use keyberon::action::{k, l, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

const RALT_SP: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: k(RAlt),
    tap: k(Space),
});

const LALT_BSP: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: k(LAlt),
    tap: k(BSpace),
});

const L1_ENTER: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(1),
    tap: k(Enter),
});
const L2_DEL: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(2),
    tap: k(Delete),
});

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<12,4,1,()> = keyberon::layout::layout!(
    {
        [Escape Q W E R T Y U I O P LBracket]
        [Tab A S D F G H J K L ; Quote]
        [LShift Z X C V B N M , . / RShift]
        [t t t LCtrl {LALT_BSP} {L2_DEL} {L1_ENTER} {RALT_SP} LGui t t t]
    }
);
