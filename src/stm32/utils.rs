/// Enables and resets peripheral clocks on various RCC registesr.
/// The first argument is a `apb1`, `ahb2` etc to specify the reg block. The second is something like
/// `tim1`, and the third is a `pac::RCC`.
macro_rules! rcc_en_reset {
    (apb1, $periph:expr, $rcc:expr) => {
        paste::paste! { cfg_if::cfg_if! {
            if #[cfg(any(feature = "f3", feature = "f4"))] {
                $rcc.apb1enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb1rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb1rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else if #[cfg(any(feature = "l4", feature = "l5", feature = "g4", feature = "wb", feature = "wl"))] {
                $rcc.apb1enr1.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb1rstr1.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb1rstr1.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else if #[cfg(feature = "g0")] {
                $rcc.apbenr1.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apbrstr1.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apbrstr1.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else {  // H7
                $rcc.apb1lenr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb1lrstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb1lrstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
                // todo: apb1h equivs
            }
            // todo: apb1enr2 on L5? Currently we only use it with USB, which is handled in
            // todo `usb.rs`.
        }}
    };
    (apb2, $periph:expr, $rcc:expr) => {
        paste::paste! { cfg_if::cfg_if! {
            if #[cfg(feature = "g0")] {
                $rcc.apbenr2.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apbrstr2.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apbrstr2.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else {
                $rcc.apb2enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.apb2rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.apb2rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }}
    };
    (apb4, $periph:expr, $rcc:expr) => {
        paste::paste! {
            $rcc.apb4enr.modify(|_, w| w.[<$periph en>]().set_bit());
            $rcc.apb4rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
            $rcc.apb4rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
        }
    };
    (ahb1, $periph:expr, $rcc:expr) => {
        paste::paste! { cfg_if::cfg_if! {
            if #[cfg(feature = "f3")] {
                $rcc.ahbenr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahbrstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahbrstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else if #[cfg(feature = "g0")] {
                $rcc.ahbenr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahbrstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahbrstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            } else {
                $rcc.ahb1enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb1rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb1rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }}
    };
    (ahb2, $periph:expr, $rcc:expr) => {
        paste::paste! { cfg_if::cfg_if! {
            if #[cfg(feature = "placeholder")] {
            } else {
                $rcc.ahb2enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb2rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb2rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }}
    };
    (ahb3, $periph:expr, $rcc:expr) => {
        paste::paste! { cfg_if::cfg_if! {
            if #[cfg(feature = "placeholder")] {
            } else {
                $rcc.ahb3enr.modify(|_, w| w.[<$periph en>]().set_bit());
                $rcc.ahb3rstr.modify(|_, w| w.[<$periph rst>]().set_bit());
                $rcc.ahb3rstr.modify(|_, w| w.[<$periph rst>]().clear_bit());
            }
        }}
    };
}

pub(crate) use rcc_en_reset;
