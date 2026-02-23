use std::sync::Arc;

use crate::value::{AnyValue, Value};

#[test]
fn round_trips() {
    let value = 13u32;
    let value = Value::from(value);
    let value = AnyValue::from(value);
    let value = value.downcast::<u32>();
    assert_eq!(*value, 13u32);
    let value = value.get();
    assert_eq!(value, 13u32);

    let value = Arc::new(13u32);
    let value = Value::from(value);
    let value = AnyValue::from(value);
    let value = value.downcast::<Arc<u32>>();
    assert_eq!(*value, 13u32);
    let value = value.get();
    assert_eq!(value, Arc::new(13u32));
}
