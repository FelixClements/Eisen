use crate::vector_runner::run_vectors_path;
use jni::objects::{JClass, JString};
use jni::sys::jint;
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_com_example_myapplication_VectorRunner_runVectors<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    json_path: JString<'local>,
    out_path: JString<'local>,
) -> jint {
    let json: String = match env.get_string(&json_path) {
        Ok(s) => s.into(),
        Err(_) => return 1,
    };
    let out: String = match env.get_string(&out_path) {
        Ok(s) => s.into(),
        Err(_) => return 1,
    };
    match run_vectors_path(&json, &out) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
