/// Syntax helper for getting global variables of the form `Mutex<RefCell<Option>>>` from an interrupt-free
/// context - eg in interrupt handlers.
///
/// Example: `access_global!(DELAY, delay, cs)`
#[macro_export]
macro_rules! access_global {
    ($NAME_GLOBAL:ident, $name_local:ident, $cs:expr) => {
        let mut part1 = $NAME_GLOBAL.borrow($cs).borrow_mut();
        let $name_local = part1.as_mut().unwrap();
    };
}

/// Similar to `access_global`, but combining multiple calls.
///
/// Example: `access_globals!([
///     (USB_DEV, usb_dev),
///     (USB_SERIAL, usb_serial),
/// ], cs);`
#[macro_export]
macro_rules! access_globals {
    ([$(($NAME_GLOBAL:ident, $name_local:ident)),* $(,)?], $cs:expr) => {
        $(
            let mut part = $NAME_GLOBAL.borrow($cs).borrow_mut();
            let $name_local = part.as_mut().unwrap();
        )*
    };
}

/// Syntax helper for setting global variables of the form `Mutex<RefCell<Option>>>`.
/// eg in interrupt handlers. Ideal for non-copy-type variables that can't be initialized
/// immediatiately.
///
/// Example: `make_globals!(
///     (USB_SERIAL, SerialPort<UsbBusType>),
///     (DELAY, Delay),
/// )`
#[macro_export]
macro_rules! make_globals {
    ($(($NAME:ident, $type:ty)),+ $(,)?) => {
        $(
            static $NAME: Mutex<RefCell<Option<$type>>> = Mutex::new(RefCell::new(None));
        )+
    };
}

/// Syntax helper for setting global variables of the form `Mutex<Cell<>>>`.
/// eg in interrupt handlers. Ideal for copy-type variables.
///
/// Example: `make_simple_globals!(
///     (VALUE, f32, 2.),
///     (SETTING, Setting, Setting::A),
/// )`
#[macro_export]
macro_rules! make_simple_globals {
    ($(($NAME:ident, $type:ty, $val:expr)),+ $(,)?) => {
        $(
            static $NAME: Mutex<Cell<$type>> = Mutex::new(Cell::new($val));
        )+
    };
}

/// In the prelude, we export helper macros.
pub mod prelude {
    pub use access_global;
    pub use access_globals;
    pub use make_globals;
    pub use make_simple_globals;
}
