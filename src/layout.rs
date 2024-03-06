use keyberon::action::Action::Trans;
use keyberon::action::{k, l, m, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

const LALT_SPACE: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: k(LAlt),
    tap: k(Space),
});

const L3_BSP: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(3),
    tap: k(BSpace),
});

const L1_ENTER: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(1),
    tap: k(Enter),
});
const L2_DELETE: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(2),
    tap: k(Delete),
});

macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k].as_slice())
    };
    ($k:ident,$v:ident) => {
        m(&[LShift, $k, $v].as_slice())
    };
}
macro_rules! a {
    ($k:ident) => {
        m(&[RAlt, $k].as_slice())
    };
}
macro_rules! m {
    ($k:ident, $v:ident) => {
        m(&[$k, $v].as_slice())
    };
}

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<12, 4, 4, ()> = [
    [
        [k(Escape),  k(X), k(V),  k(L),    k(C),     k(W),    k(K),     k(H),    k(G),    k(F),    k(Q),     k(Minus)],
        [k(Tab),     k(U), k(I),  k(A),    k(E),     k(O),    k(S),     k(N),    k(R),    k(T),    k(D),     k(Z)],
        [k(LShift),  k(LBracket), k(SColon),  k(Quote),    k(P),     k(Y),    k(B),     k(M),    k(Comma),k(Dot),  k(J), k(RShift)],
        [Trans,         Trans,    Trans,    k(LCtrl), L3_BSP,    L2_DELETE,     L1_ENTER,        LALT_SPACE, k(LGui), Trans,   Trans,    Trans      ],
    ],
    
       [
        [k(F11), k(F1),k(F2),  k(F3),  k(F4),   k(F5),    k(F6),k(F7),k(F8),    k(F9),   k(F10), k(F12)],
        [Trans, k(Kb1), k(Kb2), k(Kb3), k(Kb4), k(Kb5),  k(Kb6), k(Kb7), k(Kb8), k(Kb9), k(Kb0), Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
    ],
       [
        [Trans, Trans, Trans, Trans, Trans, Trans, k(PgUp), Trans, k(Up), Trans, Trans, Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, k(Home), k(Left), k(Down), k(Right), k(End), Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, k(PgDown), k(BSpace), k(Delete), k(Space), Trans, Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
       ],
       [
        [Trans, a!(Dot), s!(Slash), a!(Kb8), a!(Kb9), m!(Grave,Space), s!( Kb1), k(NonUsBslash), s!(NonUsBslash), s!(Kb0), s!( Kb6), a!(S)],
        [Trans, a!(Minus), s!(Kb7), a!(Kb7), a!(Kb0), s!(RBracket), s!(Minus), s!(Kb8), s!(Kb9), k(Slash), s!(SColon), a!(Q)],
        [Trans, k(NonUsHash), s!(Kb4), a!(NonUsBslash), a!(RBracket), s!(Equal, Space), k(RBracket), s!(Kb5), s!(Kb2), s!(NonUsHash), s!(Comma), Trans],
        [Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
       ]
];
