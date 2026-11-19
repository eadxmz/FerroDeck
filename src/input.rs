//! Full Remote KeyCode + ScanCode capture & action router.
//! Supports persistent custom keybinds for Voice Assistant and Control Center,
//! capturing OEM buttons even when `keyCode == 0` (`KEYCODE_UNKNOWN`) with a `scanCode`.

use std::collections::HashMap;

pub const AKEYCODE_HOME: i32 = 3;
pub const AKEYCODE_BACK: i32 = 4;
pub const AKEYCODE_DPAD_UP: i32 = 19;
pub const AKEYCODE_DPAD_DOWN: i32 = 20;
pub const AKEYCODE_DPAD_LEFT: i32 = 21;
pub const AKEYCODE_DPAD_RIGHT: i32 = 22;
pub const AKEYCODE_DPAD_CENTER: i32 = 23;
pub const AKEYCODE_ENTER: i32 = 66;
pub const AKEYCODE_DEL: i32 = 67;
pub const AKEYCODE_MENU: i32 = 82;
pub const AKEYCODE_SEARCH: i32 = 84;
pub const AKEYCODE_SETTINGS: i32 = 176;
pub const AKEYCODE_PROG_YELLOW: i32 = 185;
pub const AKEYCODE_ASSIST: i32 = 219;
pub const AKEYCODE_VOICE_ASSIST: i32 = 231;

#[inline]
pub fn pack_key_id(key_code: i32, scan_code: i32) -> u32 {
    let kc = (key_code.max(0) as u32) & 0xFFFF;
    let sc = (scan_code.max(0) as u32) & 0xFFFF;
    (sc << 16) | kc
}

pub fn friendly_key_name(key_code: i32, scan_code: i32) -> String {
    let base = match key_code {
        82 => "Menu",
        84 => "Search",
        85 => "Play / Pause",
        86 => "Media Stop",
        87 => "Next Track",
        88 => "Prev Track",
        89 => "Rewind",
        90 => "Fast Forward",
        91 | 164 => "Mute",
        165 => "Info",
        166 => "Channel Up",
        167 => "Channel Down",
        170 => "TV",
        172 => "Guide",
        176 => "Settings",
        178 => "TV Input",
        183 => "Red Button",
        184 => "Green Button",
        185 => "Yellow Button",
        186 => "Blue Button",
        187 => "Recent Apps",
        219 => "Assist",
        228 => "Last Channel",
        231 => "Voice Assist",
        24 => "Volume Up",
        25 => "Volume Down",
        0 => return format!("OEM Button (sc:{})", scan_code),
        _ => return format!("Key #{} (sc:{})", key_code, scan_code),
    };
    if scan_code > 0 {
        format!("{} (sc:{})", base, scan_code)
    } else {
        base.to_string()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaptureTarget {
    None,
    ControlCenter,
    VoiceAssistant,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundAction {
    OpenControlCenter,
    OpenSettings,
    OpenSearch,
    OpenRecentApps,
    ToggleClockStyle,
    CycleWallpaper,
    TriggerVoiceAssist,
    LaunchApp(String),
}

pub struct RemoteMapper {
    pub bindings: HashMap<u32, BoundAction>,
    pub capture_target: CaptureTarget,
    pub cc_key_label: String,
    pub voice_key_label: String,
    pub captured_voice_pair: Option<(i32, i32)>,
    pub captured_cc_pair: Option<(i32, i32)>,
    pub bound_count: usize,
}

impl RemoteMapper {
    pub fn new() -> Self {
        let mut m = Self {
            bindings: HashMap::new(),
            capture_target: CaptureTarget::None,
            cc_key_label: "Menu Button".into(),
            voice_key_label: "Yellow Mic / Voice Assist".into(),
            captured_voice_pair: None,
            captured_cc_pair: None,
            bound_count: 0,
        };
        m.reset_defaults();
        m
    }

    pub fn reset_defaults(&mut self) {
        self.bindings.clear();
        self.bindings
            .insert(pack_key_id(AKEYCODE_MENU, 0), BoundAction::OpenControlCenter);
        self.bindings
            .insert(pack_key_id(AKEYCODE_SETTINGS, 0), BoundAction::OpenSettings);
        self.bindings
            .insert(pack_key_id(AKEYCODE_PROG_YELLOW, 0), BoundAction::TriggerVoiceAssist);
        self.bindings
            .insert(pack_key_id(AKEYCODE_VOICE_ASSIST, 0), BoundAction::TriggerVoiceAssist);
        self.bindings
            .insert(pack_key_id(AKEYCODE_ASSIST, 0), BoundAction::TriggerVoiceAssist);
        self.bindings
            .insert(pack_key_id(AKEYCODE_SEARCH, 0), BoundAction::TriggerVoiceAssist);
        self.capture_target = CaptureTarget::None;
        self.cc_key_label = "Menu Button".into();
        self.voice_key_label = "Yellow Mic / Voice Assist".into();
        self.captured_voice_pair = None;
        self.captured_cc_pair = None;
        self.bound_count = self.bindings.len();
    }

    pub fn load_saved_bindings(&mut self, vkc: i32, vsc: i32, cckc: i32, ccsc: i32) {
        if vkc > 0 || vsc > 0 {
            let key_id = pack_key_id(vkc.max(0), vsc.max(0));
            if key_id != 0 {
                self.bindings.insert(key_id, BoundAction::TriggerVoiceAssist);
                self.voice_key_label = friendly_key_name(vkc.max(0), vsc.max(0));
            }
        }
        if cckc > 0 || ccsc > 0 {
            let key_id = pack_key_id(cckc.max(0), ccsc.max(0));
            if key_id != 0 {
                self.bindings.insert(key_id, BoundAction::OpenControlCenter);
                self.cc_key_label = friendly_key_name(cckc.max(0), ccsc.max(0));
            }
        }
        self.bound_count = self.bindings.len();
    }

    pub fn is_reserved_nav(key_code: i32) -> bool {
        matches!(
            key_code,
            AKEYCODE_DPAD_UP
                | AKEYCODE_DPAD_DOWN
                | AKEYCODE_DPAD_LEFT
                | AKEYCODE_DPAD_RIGHT
                | AKEYCODE_DPAD_CENTER
                | AKEYCODE_ENTER
                | AKEYCODE_BACK
        )
    }

    pub fn try_capture_key(&mut self, key_code: i32, scan_code: i32) -> Option<(CaptureTarget, String)> {
        if self.capture_target == CaptureTarget::None {
            return None;
        }
        let key_id = pack_key_id(key_code, scan_code);
        if key_id == 0 || Self::is_reserved_nav(key_code) || key_code == AKEYCODE_HOME {
            return None;
        }
        let target = self.capture_target;
        let desc = friendly_key_name(key_code, scan_code);
        match target {
            CaptureTarget::VoiceAssistant => {
                self.voice_key_label = desc.clone();
                self.bindings.insert(key_id, BoundAction::TriggerVoiceAssist);
                self.captured_voice_pair = Some((key_code, scan_code));
            }
            CaptureTarget::ControlCenter => {
                self.cc_key_label = desc.clone();
                self.bindings.insert(key_id, BoundAction::OpenControlCenter);
                self.captured_cc_pair = Some((key_code, scan_code));
            }
            CaptureTarget::None => {}
        }
        self.bound_count = self.bindings.len();
        self.capture_target = CaptureTarget::None;
        Some((target, desc))
    }

    pub fn lookup_action(&self, key_code: i32, scan_code: i32) -> Option<BoundAction> {
        let key_id = pack_key_id(key_code, scan_code);
        if key_id == 0 {
            return None;
        }
        if let Some(act) = self.bindings.get(&key_id) {
            return Some(act.clone());
        }
        if key_code > 0 {
            if let Some(act) = self.bindings.get(&pack_key_id(key_code, 0)) {
                return Some(act.clone());
            }
        }
        if scan_code > 0 {
            if let Some(act) = self.bindings.get(&pack_key_id(0, scan_code)) {
                return Some(act.clone());
            }
        }
        None
    }
}
