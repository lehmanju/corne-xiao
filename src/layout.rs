use keyberon::action::Action::Trans;
use keyberon::action::{k, l, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

const RALT_ENTER: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: k(RAlt),
    tap: k(Enter),
});

const LALT_DEL: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: k(LAlt),
    tap: k(Delete),
});

const L1_SP: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(1),
    tap: k(Space),
});
const L2_BSP: Action = Action::HoldTap(&HoldTapAction {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: l(2),
    tap: k(BSpace),
});

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<12, 4, 3, ()> = [
    [
        [k(Escape),  k(Q), k(W),  k(E),    k(R),     k(T),    k(Y),     k(U),    k(I),    k(O),    k(P),     k(LBracket)],
        [k(Tab),     k(A), k(S),  k(D),    k(F),     k(G),    k(H),     k(J),    k(K),    k(L),    k(SColon),k(Quote)   ],
        [k(LShift),  k(Z), k(X),  k(C),    k(V),     k(B),    k(N),     k(M),    k(Comma),k(Dot),  k(Slash), k(RShift)  ],
        [Trans,         Trans,    Trans,    k(LCtrl), LALT_DEL,    L2_BSP,     L1_SP,        RALT_ENTER, k(LGui), Trans,   Trans,    Trans      ],
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
       ]
];
