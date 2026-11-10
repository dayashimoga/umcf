use umcf_llama_sys::*;

#[test]
fn test_llama_backend_init_and_device_enumeration() {
    // SAFETY: Verified safe C calls to backend initialization and device query
    unsafe {
        llama_backend_init();
        let dev_count = ggml_backend_dev_count();
        assert!(dev_count >= 1);
        let dev = ggml_backend_dev_get(0);
        assert!(!dev.is_null());
        let name_ptr = ggml_backend_dev_name(dev);
        assert!(!name_ptr.is_null());
        let name = std::ffi::CStr::from_ptr(name_ptr)
            .to_str()
            .expect("valid utf8 name");
        assert_eq!(name, "CPU");
        llama_backend_free();
    }
}
