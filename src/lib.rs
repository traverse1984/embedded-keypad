#![warn(clippy::all)]
#![no_std]

use embedded_digi as digi;
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub trait Keypad {
    /// Returns true if any key is pressed, without trying to read which key(s).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// println!("Press any key...");
    ///
    /// loop {
    ///     if keypad.key_is_pressed() {
    ///         println!("Thanks!");
    ///         break;
    ///     }
    /// }
    /// ```
    fn key_is_pressed(&self) -> bool;

    /// Read a single key press from the keypad. The first key identified is
    /// returned as [Some]. If no key is pressed, [None] is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    ///
    /// match keypad.read() {
    ///     Some(key) => println!("Got key: {}.", key),
    ///     None => println!("No key pressed.");
    /// }
    /// ```
    fn read(&mut self) -> Option<u8>;

    /// Read multiple key presses from the keypad. Up to four keys can be
    /// identified at once, but it is not possible to detect two keys from
    /// the same row or column. The identified [Keys] are returned as [Some].
    /// If no keys are pressed, [None] is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let val = match keypad.read_multi() {
    ///     Some(Keys::One(key)) => println!("Got key: {}.", key),
    ///     Some(Keys::Two(key, ctrl)) if ctrl == 0xC => {
    ///         println!("Got ctrl key: {}.", key);
    ///     }
    ///     Some(_) => println!("Invalid key combination."),
    ///     None => println!("No key pressed."),
    /// }
    ///
    /// ```
    fn read_multi(&mut self) -> Option<Keys>;
}

/// One or more keys pressed simultaneously.
#[derive(Debug, Clone, Copy)]
pub enum Keys {
    One(u8),
    Two(u8, u8),
    Three(u8, u8, u8),
    Four(u8, u8, u8, u8),
}

impl Keys {
    /// Convert the keys to an array of `Option<u8>`.
    pub fn as_array(&self) -> [Option<u8>; 4] {
        use Keys::*;

        match self {
            &One(k0) => [Some(k0), None, None, None],
            &Two(k0, k1) => [Some(k0), Some(k1), None, None],
            &Three(k0, k1, k2) => [Some(k0), Some(k1), Some(k2), None],
            &Four(k0, k1, k2, k3) => [Some(k0), Some(k1), Some(k2), Some(k3)],
        }
    }

    /// Determines whether a given key is among those pressed.
    pub fn includes(&self, key: u8) -> bool {
        use Keys::*;

        match self {
            &One(k0) => k0 == key,
            &Two(k0, k1) => k0 == key || k1 == key,
            &Three(k0, k1, k2) => k0 == key || k1 == key || k2 == key,
            &Four(k0, k1, k2, k3) => k0 == key || k1 == key || k2 == key || k3 == key,
        }
    }
}

/// A keypad implemented using eight GPIO pins.
pub struct GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    col1: C1,
    col2: C2,
    col3: C3,
    col4: C4,
    row1: R1,
    row2: R2,
    row3: R3,
    row4: R4,
    keymap: [[u8; 4]; 4],
}

impl<C1, C2, C3, C4, R1, R2, R3, R4> GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    const DEFAULT_KEYMAP: [[u8; 4]; 4] = [
        [0x1, 0x2, 0x3, 0xF],
        [0x4, 0x5, 0x6, 0xE],
        [0x7, 0x8, 0x9, 0xD],
        [0xA, 0x0, 0xB, 0xC],
    ];

    pub fn new(
        col1: C1,
        col2: C2,
        col3: C3,
        col4: C4,
        row1: R1,
        row2: R2,
        row3: R3,
        row4: R4,
    ) -> Self {
        let mut keypad = Self {
            col1,
            col2,
            col3,
            col4,
            row1,
            row2,
            row3,
            row4,
            keymap: Self::DEFAULT_KEYMAP,
        };

        keypad.reset();
        keypad
    }

    pub fn with_keymap(mut self, keymap: [[u8; 4]; 4]) -> Self {
        self.keymap = keymap;
        self
    }

    fn reset(&mut self) {
        digi::write!(self.col1, self.col2, self.col3, self.col4 => true);
    }

    fn read_char(&self, col: usize) -> Option<u8> {
        let key = digi::read!(u8; self.row4, self.row3, self.row2, self.row1);

        match key {
            8..=15 => Some(3),
            4..=7 => Some(2),
            2 | 3 => Some(1),
            1 => Some(0),
            _ => None,
        }
        .map(|row| self.keymap[row][col])
    }
}

impl<C1, C2, C3, C4, R1, R2, R3, R4> Keypad for GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    fn key_is_pressed(&self) -> bool {
        digi::read!(any true; self.row4, self.row3, self.row2, self.row1)
    }

    fn read(&mut self) -> Option<u8> {
        if !self.key_is_pressed() {
            return None;
        }

        for pos in 0..4 {
            digi::write!(self.col4, self.col3, self.col2, self.col1 => 4 bit => 1 << pos);

            if let Some(key) = self.read_char(pos) {
                self.reset();
                return Some(key);
            }
        }

        self.reset();
        None
    }

    fn read_multi(&mut self) -> Option<Keys> {
        if !self.key_is_pressed() {
            return None;
        }

        let mut count = 0;
        let mut buf = [0u8; 4];

        for pos in 0..4 {
            digi::write!(self.col4, self.col3, self.col2, self.col1 => 4 bit => 1 << pos);

            self.read_char(pos).map(|key| {
                buf[count] = key;
                count += 1;
            });
        }

        self.reset();

        match count {
            0 => None,
            1 => Some(Keys::One(buf[0])),
            2 => Some(Keys::Two(buf[0], buf[1])),
            3 => Some(Keys::Three(buf[0], buf[1], buf[2])),
            4 => Some(Keys::Four(buf[0], buf[1], buf[2], buf[3])),
            _ => unreachable!(),
        }
    }
}
