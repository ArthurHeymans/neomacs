//! Neomacs-specific defaults for the optional mimalloc global allocator.

cfg_select! {
    target_os = "linux" => {
        // This experimental option has enum value 4 in both mimalloc v2 and
        // v3. libmimalloc-sys intentionally leaves experimental option names
        // out of its Rust bindings, while still exposing the option API.
        const MI_OPTION_ARENA_EAGER_COMMIT: libmimalloc_sys::mi_option_t = 4;

        // mimalloc's Linux process initializer uses ELF constructor priority
        // 101. Run immediately before it so `set_default` takes effect before
        // mimalloc initializes its options from the environment. The linker
        // orders `.init_array.N` sections numerically.
        #[used]
        #[unsafe(link_section = ".init_array.000100")]
        static CONFIGURE_BEFORE_MIMALLOC: unsafe extern "C" fn() = configure_before_mimalloc;

        unsafe extern "C" fn configure_before_mimalloc() {
            // Linux's overcommit support makes mimalloc eagerly commit each
            // arena by default. Doom startup measurements show that this
            // retains substantially more resident memory than commit-on-use.
            //
            // SAFETY: the platform loader invokes constructors serially before
            // main or thread creation. This function performs no allocation,
            // and mimalloc's later priority-101 initializer remains free to
            // replace the default from MIMALLOC_ARENA_EAGER_COMMIT.
            unsafe {
                libmimalloc_sys::mi_option_set_default(MI_OPTION_ARENA_EAGER_COMMIT, 0);
            }
        }
    }
    _ => {}
}
