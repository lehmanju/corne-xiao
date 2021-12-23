use keyberon::action::{k, l, m, Action::*, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

const CUT: Action = m(&[LShift, Delete]);
const COPY: Action = m(&[LCtrl, Insert]);
const PASTE: Action = m(&[LShift, Insert]);

const RALT_SP: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &k(RAlt),
    tap: &k(Space),
};

const LALT_BSP: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &k(LAlt),
    tap: &k(BSpace),
};

const L1_ENTER: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &l(1),
    tap: &k(Enter),
};
const L2_DEL: Action = HoldTap {
    timeout: 200,
    tap_hold_interval: 0,
    config: HoldTapConfig::HoldOnOtherKeyPress,
    hold: &l(2),
    tap: &k(Delete),
};

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<()> = &[
    &[
        &[k(Escape),  k(Q), k(W),  k(E),    k(R),     k(T),    k(Y),     k(U),    k(I),    k(O),    k(P),     k(LBracket)],
        &[k(Tab),     k(A), k(S),  k(D),    k(F),     k(G),    k(H),     k(J),    k(K),    k(L),    k(SColon),k(Quote)   ],
        &[k(LShift),  k(Z), k(X),  k(C),    k(V),     k(B),    k(N),     k(M),    k(Comma),k(Dot),  k(Slash), k(RShift)  ],
        &[Trans,      Trans,Trans, k(LCtrl),L2_DEL,   LALT_BSP,RALT_SP,  L1_ENTER,k(LGui), Trans,   Trans,    Trans      ],
    ],
       &[
        &[k(F1),k(F2),  k(F3),  k(F4),   k(F5),    k(F6),k(F7),k(F8),    k(F9),   k(F10), k(F11), k(F12)],
        &[Trans, k(Kb1), k(Kb2), k(Kb3), k(Kb4), k(Kb5),  k(Kb6), k(Kb7), k(Kb8), k(Kb9), k(Kb0), Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
        &[Trans, Trans, Trans, Custom(()), Trans, Trans, Trans, Trans, Custom(()), Trans, Trans, Trans],
    ],
       &[
        &[Trans, Trans, k(MediaPreviousSong), k(MediaPlayPause), k(MediaNextSong), Trans, k(PgUp), Trans, k(Up), Trans, Trans, Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, k(Home), k(Left), k(Down), k(Right), k(End), Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, k(PgDown), k(BSpace), k(Delete), k(Space), Trans, Trans],
        &[Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans, Trans],
       ]
];
