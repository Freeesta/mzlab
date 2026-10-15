// The value type of the `[review]` table, the conversion of the setting types and the `settings!` macro that declares them.

/// A value of the `[review]` table: a number, a text or a list of them.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    List(Vec<Value>),
}

macro_rules! settings {
    ($( $(#[$doc:meta])* $name:ident : $ty:ty = $def:expr ;)*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct ReviewSettings { $( $(#[$doc])* pub $name: $ty, )* }

        impl Default for ReviewSettings {
            fn default() -> Self { ReviewSettings { $( $name: $def, )* } }
        }

        impl ReviewSettings {
            /// The keys a campaign may set.
            pub const KEYS: &'static [&'static str] = &[ $( stringify!($name), )* ];

            fn set(&mut self, key: &str, v: &Value) -> Result<(), String> {
                match key {
                    $( stringify!($name) => {
                        self.$name = <$ty as Field>::from_value(v).ok_or_else(|| format!("[review] {key}: wrong type"))?;
                        Ok(())
                    } )*
                    _ => Err(format!("[review]: unknown key {key}")),
                }
            }

            /// Every setting with its value, in declaration order.
            pub fn entries(&self) -> Vec<(&'static str, Value)> {
                vec![ $( (stringify!($name), Field::to_value(&self.$name)), )* ]
            }
        }
    };
}

pub(super) use settings;

pub(super) trait Field: Sized {
    fn from_value(v: &Value) -> Option<Self>;
    fn to_value(&self) -> Value;
}

impl Field for f64 {
    fn from_value(v: &Value) -> Option<f64> {
        if let Value::Num(n) = v {
            Some(*n)
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Num(*self)
    }
}

impl Field for i64 {
    fn from_value(v: &Value) -> Option<i64> {
        if let Value::Num(n) = v {
            (n.fract() == 0.0).then_some(*n as i64)
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Num(*self as f64)
    }
}

impl Field for String {
    fn from_value(v: &Value) -> Option<String> {
        if let Value::Str(s) = v {
            Some(s.clone())
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Str(self.clone())
    }
}

impl<T: Field> Field for Vec<T> {
    fn from_value(v: &Value) -> Option<Vec<T>> {
        if let Value::List(l) = v {
            l.iter().map(T::from_value).collect()
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::List(self.iter().map(Field::to_value).collect())
    }
}
