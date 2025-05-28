use crate::frb_generated::StreamSink;
use windows::core::{Result, HSTRING};
use windows::{
    Foundation::{self, TypedEventHandler},
    Media::{
        AutoRepeatModeChangeRequestedEventArgs, MediaPlaybackAutoRepeatMode, MediaPlaybackType,
        PlaybackPositionChangeRequestedEventArgs, ShuffleEnabledChangeRequestedEventArgs,
        SystemMediaTransportControls, SystemMediaTransportControlsButton,
        SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    Storage::{StorageFile, Streams::RandomAccessStreamReference},
};

use super::{
    config::SMTCConfig, metadata::MusicMetadata, playback_status::PlaybackStatus,
    timeline::PlaybackTimeline,
};

#[derive(Debug, Clone)]
pub struct SMTCInternal {
    pub media_player: Box<windows::Media::Playback::MediaPlayer>,
}

impl SMTCInternal {
    pub fn new(enabled: Option<bool>) -> anyhow::Result<Self> {
        let media_player = Box::new(windows::Media::Playback::MediaPlayer::new()?);

        let smtc = media_player.SystemMediaTransportControls()?;

        media_player.CommandManager()?.SetIsEnabled(false)?;

        smtc.SetIsEnabled(enabled.unwrap_or(true))?;
        Ok(Self { media_player })
    }

    pub fn update_config(&self, config: SMTCConfig) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.SetIsPlayEnabled(config.play_enabled)?;
        smtc.SetIsPauseEnabled(config.pause_enabled)?;
        smtc.SetIsNextEnabled(config.next_enabled)?;
        smtc.SetIsPreviousEnabled(config.prev_enabled)?;
        smtc.SetIsFastForwardEnabled(config.fast_forward_enabled)?;
        smtc.SetIsRewindEnabled(config.rewind_enabled)?;
        smtc.SetIsStopEnabled(config.stop_enabled)?;

        Ok(())
    }

    pub fn update_metadata(
        &self,
        metadata: MusicMetadata,
        app_id: Option<String>,
    ) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        let updater = smtc.DisplayUpdater()?;

        updater.ClearAll()?;

        app_id.map(|s| updater.SetAppMediaId(&HSTRING::from(s)));

        updater.SetType(MediaPlaybackType::Music)?;

        let music_properties = updater.MusicProperties()?;

        metadata.h_artist().map(|s| music_properties.SetArtist(&s));

        metadata
            .h_album()
            .map(|s| music_properties.SetAlbumTitle(&s));
        metadata.h_title().map(|s| music_properties.SetTitle(&s));
        metadata
            .h_album_artist()
            .map(|s| music_properties.SetAlbumArtist(&s));

        let thumbnail = if let Some(s) = metadata.h_thumbnail() {
            let is_url = metadata.h_thumbnail_raw().starts_with("http");
            if is_url {
                let uri = Foundation::Uri::CreateUri(&s).unwrap();
                Some(RandomAccessStreamReference::CreateFromUri(&uri).unwrap())
            } else {
                fn get_storage_file_sync(path: &HSTRING) -> Result<StorageFile> {
                    let async_op = StorageFile::GetFileFromPathAsync(path)?;
                    async_op.get()
                }
                match get_storage_file_sync(&s) {
                    Ok(file) => Some(RandomAccessStreamReference::CreateFromFile(&file).unwrap()),
                    Err(_e) => None,
                }
            }
        } else {
            None
        };

        match thumbnail {
            Some(x) => updater.SetThumbnail(&x)?,
            None => updater.SetThumbnail(None)?,
        }

        updater.Update()?;

        Ok(())
    }

    pub fn clear_metadata(&self) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        let updater = smtc.DisplayUpdater()?;

        updater.ClearAll()?;
        updater.Update()?;

        Ok(())
    }

    pub fn update_timeline(&self, timeline: PlaybackTimeline) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        let timeline_properties: anyhow::Result<SystemMediaTransportControlsTimelineProperties> =
            timeline.into();
        smtc.UpdateTimelineProperties(&timeline_properties?)?;

        Ok(())
    }

    pub fn update_playback_status(&self, status: PlaybackStatus) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.SetPlaybackStatus(status.into())?;
        Ok(())
    }

    pub fn update_shuffle(&self, shuffle: bool) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;
        smtc.SetShuffleEnabled(shuffle)?;
        Ok(())
    }

    pub fn update_repeat_mode(&self, repeat_mode: String) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        match repeat_mode.as_str() {
            "none" => smtc.SetAutoRepeatMode(MediaPlaybackAutoRepeatMode::None)?,
            "track" => smtc.SetAutoRepeatMode(MediaPlaybackAutoRepeatMode::Track)?,
            "list" => smtc.SetAutoRepeatMode(MediaPlaybackAutoRepeatMode::List)?,
            _ => smtc.SetAutoRepeatMode(MediaPlaybackAutoRepeatMode::None)?,
        }

        Ok(())
    }

    pub fn enable_smtc(&self) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls();
        smtc?.SetIsEnabled(true)?;
        Ok(())
    }

    pub fn disable_smtc(&self) -> anyhow::Result<()> {
        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls();
        smtc?.SetIsEnabled(false)?;
        Ok(())
    }

    pub fn button_press_event(&self, sink: StreamSink<String>) -> anyhow::Result<()> {
        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            let button = args.as_ref().unwrap().Button().unwrap();

            match button {
                SystemMediaTransportControlsButton::Play => {
                    sink.add("play".to_string());
                }
                SystemMediaTransportControlsButton::Pause => {
                    sink.add("pause".to_string());
                }
                SystemMediaTransportControlsButton::Next => {
                    sink.add("next".to_string());
                }
                SystemMediaTransportControlsButton::Previous => {
                    sink.add("previous".to_string());
                }
                SystemMediaTransportControlsButton::FastForward => {
                    sink.add("fast_forward".to_string());
                }
                SystemMediaTransportControlsButton::Rewind => {
                    sink.add("rewind".to_string());
                }
                SystemMediaTransportControlsButton::Stop => {
                    sink.add("stop".to_string());
                }
                SystemMediaTransportControlsButton::Record => {
                    sink.add("record".to_string());
                }
                SystemMediaTransportControlsButton::ChannelUp => {
                    sink.add("channel_up".to_string());
                }
                SystemMediaTransportControlsButton::ChannelDown => {
                    sink.add("channel_down".to_string());
                }
                _ => {}
            }
            Ok(())
        });

        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.ButtonPressed(&handler)?;

        anyhow::Result::Ok(())
    }

    pub fn position_change_request_event(&self, sink: StreamSink<i64>) -> anyhow::Result<()> {
        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            PlaybackPositionChangeRequestedEventArgs,
        >::new(move |_, args| {
            let position_ms = args
                .as_ref()
                .unwrap()
                .RequestedPlaybackPosition()
                .unwrap()
                .Duration;

            sink.add(position_ms);
            Ok(())
        });

        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.PlaybackPositionChangeRequested(&handler)?;

        anyhow::Result::Ok(())
    }

    pub fn shuffle_request_event(&self, sink: StreamSink<bool>) -> anyhow::Result<()> {
        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            ShuffleEnabledChangeRequestedEventArgs,
        >::new(move |_, args| {
            let shuffle = args.as_ref().unwrap().RequestedShuffleEnabled().unwrap();

            sink.add(shuffle);
            Ok(())
        });

        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.ShuffleEnabledChangeRequested(&handler)?;

        anyhow::Result::Ok(())
    }

    pub fn repeat_mode_request_event(&self, sink: StreamSink<String>) -> anyhow::Result<()> {
        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            AutoRepeatModeChangeRequestedEventArgs,
        >::new(move |_, args| {
            let repeat_mode = args.as_ref().unwrap().RequestedAutoRepeatMode().unwrap();

            match repeat_mode {
                MediaPlaybackAutoRepeatMode::None => {
                    sink.add("none".to_string());
                }
                MediaPlaybackAutoRepeatMode::Track => {
                    sink.add("track".to_string());
                }
                MediaPlaybackAutoRepeatMode::List => {
                    sink.add("list".to_string());
                }
                _ => {
                    sink.add("none".to_string());
                }
            }

            Ok(())
        });

        let media_player = &self.media_player;
        let smtc = media_player.SystemMediaTransportControls()?;

        smtc.AutoRepeatModeChangeRequested(&handler)?;

        anyhow::Result::Ok(())
    }
}
