//! Full Remote KeyCode + ScanCode capture & action router.
//! Captures OEM keys (including the yellow mic / voice assistant button) even when
//! Android assigns `keyCode == 0` (`KEYCODE_UNKNOWN`) with a hardware `scanCode`.

use std::collections::HashMap;

pub const AKEYCODE_HOME: i32 = 3;
pub const AKEYCODE_BACK: i32 = 4;
pub const AKEYCODE_DPAD_UP: i32 = 19;
pub const AKEYCODE_DPAD_DOWN: i32 = 20;
pub const AKEYCODE_DPAD_LEFT: i32 = 21;
pub const AKEYCODE_DPAD_RIGHT: i32 = 22;
pub const AKEYCODE_DPAD_CENTER: i32 = 23;
pub const AKEYCODE_ENTER: i32 = 66;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundAction {
    OpenSettingsPill,
    ToggleClockStyle,
    CycleWallpaper,
    TriggerVoiceAssist,
    LaunchApp(String),
}

pub struct RemoteMapper {
    pub bindings: HashMap<u32, BoundAction>,
    pub capture_mode: bool,
    pub last_captured_key_id: Option<u32>,
    pub last_captured_label: String,
}

impl RemoteMapper {
    pub fn new() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(pack_key_id(AKEYCODE_MENU, 0), BoundAction::OpenSettingsPill);
        bindings.insert(pack_key_id(AKEYCODE_SETTINGS, 0), BoundAction::OpenSettingsPill);
        bindings.insert(pack_key_id(AKEYCODE_PROG_YELLOW, 0), BoundAction::TriggerVoiceAssist);
        bindings.insert(pack_key_id(AKEYCODE_VOICE_ASSIST, 0), BoundAction::TriggerVoiceAssist);
        bindings.insert(pack_key_id(AKEYCODE_ASSIST, 0), BoundAction::TriggerVoiceAssist);
        bindings.insert(pack_key_id(AKEYCODE_SEARCH, 0), BoundAction::TriggerVoiceAssist);
        Self {
            bindings,
            capture_mode: false,
            last_captured_key_id: None,
            last_captured_label: "Press OK then remote key".into(),
        }
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

    pub fn on_raw_key(&mut self, key_code: i32, scan_code: i32) -> Option<BoundAction> {
        let key_id = pack_key_id(key_code, scan_code);
        if key_id == 0 {
            return None;
        }

        if self.capture_mode && !Self::is_reserved_nav(key_code) && key_code != AKEYCODE_HOME {
            self.last_captured_key_id = Some(key_id);
            self.last_captured_label = format!("Bound kc={} sc={}", key_code, scan_code);
            self.bindings.insert(key_id, BoundAction::OpenSettingsPill);
            self.capture_mode = false;
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
