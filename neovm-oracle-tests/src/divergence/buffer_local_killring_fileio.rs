//! Divergence tests: buffer-local variables, defaults, and kill ring.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_buffer_local_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (42 0 t 42)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defvar my-bl-test 0)
  (set (make-local-variable 'my-bl-test) 42)
  (list my-bl-test
        (default-value 'my-bl-test)
        (local-variable-p 'my-bl-test)
        (buffer-local-value 'my-bl-test (current-buffer))))"#,
        expect,
    );
}

#[test]
fn divergence_kill_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (0 nil 0)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defvar my-bl-kill 0)
  (set (make-local-variable 'my-bl-kill) 99)
  (kill-local-variable 'my-bl-kill)
  (list my-bl-kill
        (local-variable-p 'my-bl-kill)
        (default-value 'my-bl-kill)))"#,
        expect,
    );
}

#[test]
fn divergence_default_toplevel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (20 20)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defvar my-dt-var 10)
  (setq-default my-dt-var 20)
  (list my-dt-var
        (default-value 'my-dt-var)))"#,
        expect,
    );
}

#[test]
fn divergence_buffer_local_value_across_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (0 55 0)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defvar my-cross-buf-var 0)
  (let ((buf (generate-new-buffer " *cross-buf-test*")))
    (with-current-buffer buf
      (set (make-local-variable 'my-cross-buf-var) 55))
    (prog1
        (list my-cross-buf-var
              (buffer-local-value 'my-cross-buf-var buf)
              (default-value 'my-cross-buf-var))
      (kill-buffer buf))))"#,
        expect,
    );
}

#[test]
fn divergence_kill_ring_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (\"third\" \"second\" \"first\" 3)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(let ((kill-ring nil))
  (kill-new "first")
  (kill-new "second")
  (kill-new "third")
  (list (car kill-ring)
        (nth 1 kill-ring)
        (nth 2 kill-ring)
        (length kill-ring)))"#,
        expect,
    );
}

#[test]
fn divergence_kill_ring_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (\"b\" \"b\" \"b\")""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(let ((kill-ring nil))
  (kill-new "a")
  (kill-new "b" t)
  (list (car kill-ring)
        (current-kill 0)
        (current-kill 1)))"#,
        expect,
    );
}

#[test]
fn divergence_kill_ring_max_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (3 \"4\" \"3\" \"2\")""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(let ((kill-ring nil)
        (kill-ring-max 3))
  (dotimes (i 5)
    (kill-new (number-to-string i)))
  (list (length kill-ring)
        (car kill-ring)
        (nth 1 kill-ring)
        (nth 2 kill-ring)))"#,
        expect,
    );
}

#[test]
fn divergence_with_temp_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t \"hello\")""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(let ((tmp (make-temp-file "neovm-test-")))
  (unwind-protect
      (progn
        (write-region "hello" nil tmp nil 'silent)
        (list (file-exists-p tmp)
              (file-readable-p tmp)
              (file-writable-p tmp)
              (with-temp-buffer
                (insert-file-contents tmp)
                (buffer-string))))
    (delete-file tmp)))"#,
        expect,
    );
}

#[test]
fn divergence_expand_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect =
        expect_test::expect![[r#""OK (\"/foo/bar/\" \"baz.el\" \"gz\" \"test\" \"file\")""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(list
  (file-name-directory "/foo/bar/baz.el")
  (file-name-nondirectory "/foo/bar/baz.el")
  (file-name-extension "test.tar.gz")
  (file-name-sans-extension "test.el")
  (file-name-base "/path/to/file.el"))"#,
        expect,
    );
}

#[test]
fn divergence_directory_files() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (\"a.txt\" \"b.txt\")""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(let ((tmp (make-temp-file "neovm-dir-test-" t)))
  (unwind-protect
      (progn
        (write-region "a" nil (expand-file-name "a.txt" tmp) nil 'silent)
        (write-region "b" nil (expand-file-name "b.txt" tmp) nil 'silent)
        (sort (directory-files tmp nil "\\.txt$") #'string<))
    (delete-directory tmp t)))"#,
        expect,
    );
}

#[test]
fn divergence_env_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""OK (\"/home/exec\" \"/home/exec/.exec/bin:/home/exec/.cargo/bin:/home/exec/.opencode/bin:/home/exec/.bun/bin:/home/exec/.config/guix/current/bin:/home/exec/.exec/bin:/home/exec/.cargo/bin:/home/exec/.opencode/bin:/home/exec/.bun/bin:/nix/store/7cr22v5sp848rac53yxnaq108600vlzg-wpewebkit-2.50.4/libexec/wpe-webkit-2.0:/nix/store/qd70v8g0561vm8m33kmnp79z00cgyi5n-gcc-wrapper-15.2.0/bin:/nix/store/sanx9fg8mry8mq92zhlm5qvb83qlxrlx-gcc-15.2.0/bin:/nix/store/pf30k3mg7n6bibc1k6609gyq7glk00k2-glibc-2.42-61-bin/bin:/nix/store/jjxngswsb214vb58qx485jhmilf0kxxy-coreutils-9.10/bin:/nix/store/kfwagnh6i1mysf7vxq679rzh30z9zj3g-binutils-wrapper-2.46/bin:/nix/store/p2vkw5s89ff1fs2d2rxqxiqil9s0jpcm-binutils-2.46/bin:/nix/store/i7scgyv862acinjghirnjzfscih1m5h9-rust-default-1.95.0/bin:/nix/store/7h9agqfhy1lyljwzbp1695j8pirip2ks-rust-cbindgen-0.29.2/bin:/nix/store/v7mjkia7ki79s5i24ldbzq1khalhgzk0-pkg-config-wrapper-0.29.2/bin:/nix/store/mw4gasdvwgscgpxpzihjgchfhs3hhqhn-clang-wrapper-21.1.8/bin:/nix/store/wm8ldw3xc402xyihlk43vlgl7knhr6g4-clang-21.1.8/bin:/nix/store/vr4sjc5ajni6j76wqkvkx84q141270ak-binutils-wrapper-2.46/bin:/nix/store/66lksljlljdd5ppgvfk8g89y8xgqcxd7-patchelf-0.15.2/bin:/nix/store/7kcshxa6bywdl3284xfkd7mi4ib27caz-compiler-rt-libc-21.1.8/bin:/nix/store/whrp41kk2qd790h8xg4v4giw5z0cmxa7-ncurses-6.6-dev/bin:/nix/store/2iaawa9vbqas51lgpn4cjnnfdv74x8fn-ncurses-6.6/bin:/nix/store/yh94s4zzq1qh0320fks90ma2lyas8rs9-nettle-3.10.2-dev/bin:/nix/store/lvkl65gd1msllc8k0qd55sf8ff1ay18x-gnutls-3.8.12-bin/bin:/nix/store/6sirsgrlw5wpjb61v07sbgs4sfsfdwx6-libxml2-2.15.1-dev/bin:/nix/store/2b9b04irbcmasriarwwgqhby01mzzwr1-libxml2-2.15.1-bin/bin:/nix/store/r7bp82svf04jqw3x7wnjlyr951jkf85k-freetype-2.14.2-dev/bin:/nix/store/zj6r42syyswkhrr174bzppj3n7xhq936-bzip2-1.0.8-bin/bin:/nix/store/mj1k1nsdqr0mp9wsnkg7blgh3xf5wssv-brotli-1.2.0/bin:/nix/store/h176f4dhbcpj4lpf8sn28vdqp1mks5jk-libpng-apng-1.6.56-dev/bin:/nix/store/v18drszzvspk1wlq06r68nxgpn2b4cvd-fontconfig-2.17.1-bin/bin:/nix/store/ndr1qrjb4y4p66b51sf49x95mymmvr2l-harfbuzz-12.3.0-dev/bin:/nix/store/qik1sfr8z8w4ffrd21yv76z2nvwyhmn5-graphite2-1.3.14/bin:/nix/store/s8fxnpfh8p3rg3byxh2zjw8gxwqsji5v-libotf-0.9.16-dev/bin:/nix/store/21m9i67v6kl36l4s8jxcm95fkb7pfai1-libotf-0.9.16/bin:/nix/store/8zhjvm4vixgvg089nn4wv7hhxlp7qg2c-cairo-1.18.4-dev/bin:/nix/store/kw0yjwbvw6arwgwaa3p8rz46qsgy4626-glib-2.86.3-dev/bin:/nix/store/ypj27q94ay0ybq9aa14gk0cxjv9d7z4m-gettext-1.0/bin:/nix/store/b9jcqjd8gnxr87p7wc91lmbyd90kzlc1-glib-2.86.3-bin/bin:/nix/store/2vnars959wqifbbkgpm9742r2k8j4a45-gstreamer-1.26.11-bin/bin:/nix/store/sqvm6lz2lbn43zflil7rgn9p0d86gpfd-libdrm-2.4.131-bin/bin:/nix/store/cl0shrxzs1h3pzgfnn94yf8rd7823wnn-gst-plugins-base-1.26.11/bin:/nix/store/xbd4z2d5ddl5w7cxjbpvb245jddbg37c-gst-plugins-bad-1.26.11/bin:/nix/store/qb16ld2dpjh2hwdx1djkd5hbyhnxrkd4-gst-plugins-rs-0.14.4/bin:/nix/store/v70k3ch8rcw9b0la3axqb34dkyxqnx2s-libjpeg-turbo-3.1.4-bin/bin:/nix/store/cbdy1d44cqa9j7x0ga72dqsk4p49ih70-libtiff-4.7.1-bin/bin:/nix/store/wp2xv937c5fn1f2zwy5clp9dd228ls0j-giflib-5.2.2/bin:/nix/store/53i0wb0l7zs8gm6mv3df3yqvmmg23kk3-gdk-pixbuf-2.44.5-dev/bin:/nix/store/pkyvhnszc7h5nncashff6xqykb7d20zx-gdk-pixbuf-2.44.5/bin:/nix/store/4cvbp49wlc9d7s6rivyz2s4fg8rrz9wd-librsvg-2.61.4/bin:/nix/store/vdz5z5d4qvsfqdafihrfwzi5r7wr24lk-libwebp-1.6.0/bin:/nix/store/nrq3pjzsjd4w9vcpgk4a2wfjlqz4xxzw-openjpeg-2.5.4/bin:/nix/store/xiq38z94b68c8dgj7nfx9xlh2984c2mp-lcms2-2.18-bin/bin:/nix/store/7xvb5060qcf36ncp47wz62rl3fsccv1g-curl-8.19.0-dev/bin:/nix/store/dgdzsx6i729gcp1rrz85zbaacgl86gab-krb5-1.22.1-dev/bin:/nix/store/qp2qzmh67rqy6i36sh3iqznk1akiw4q1-krb5-1.22.1/bin:/nix/store/yvxyaqh3bzj7nr64zlr1axyf76fgcszb-nghttp2-1.68.1/bin:/nix/store/a327a5lqzwakcs3yjgx4sa1931fph5gf-libidn2-2.3.8-bin/bin:/nix/store/2di90l89y2ygdy3rbws7dhg9nrvd3pnx-openssl-3.6.1-bin/bin:/nix/store/79kr7fafcvvmch13cyczpckz40159pk5-libpsl-0.21.5/bin:/nix/store/91jddg4g6788ilnk3kww8j8jhxhzk6d3-zstd-1.5.7-bin/bin:/nix/store/k0rqiflg1vkn1kj96br5pfxj40p3srz4-zstd-1.5.7/bin:/nix/store/sm2nq18jjqp4x0sxpl6lrvwl9rx6mvj2-curl-8.19.0-bin/bin:/nix/store/xafb1qxw69j6fg1s8ln2drppm2zjjfr5-nss-3.112.3-dev/bin:/nix/store/3qfr4jp73jac5rnkx8xj58whv4yc80zy-nspr-4.38.2-dev/bin:/nix/store/yanmwp5f435ing2nbhwa4v0gdmpl2an1-dbus-1.16.2-lib/bin:/nix/store/g6a7agib4hbnvqcny05fk8dfjplw8nkb-dbus-1.16.2/bin:/nix/store/67cm7qx8s210dwkq64vqbf3q9z62ddyg-sqlite-3.51.2-bin/bin:/nix/store/44rldadal7sqwlnmcskhgw10m7vvkcxj-tree-sitter-0.26.8/bin:/nix/store/3innqpmxwvmr2vc8h51g47aqdl6zj2b4-alsa-lib-1.2.15.3/bin:/nix/store/8w64dm3sny77mnf8jm5n1n57d1fk25x4-libselinux-3.10-bin/bin:/nix/store/0c0xdj7xpilqfy2p33l1jm407f01652w-libxkbcommon-1.13.1/bin:/nix/store/ind838l07r4zgccwhl0vmg45z94vs0fj-mesa-26.0.5/bin:/nix/store/4cisxl541h1z7rj10pvf239kkxxgh0g2-wpewebkit-2.50.4-dev/bin:/nix/store/w8gcwbngriwj0snsavhw398zk8ljgng2-weston-15.0.0/bin:/nix/store/iinwi0yijx1i309byvpafkah5vq2gla3-xdg-dbus-proxy-0.1.6/bin:/nix/store/dcqjv5cbfjk4rml9h6qw0ybyagf8n2xm-libxpm-3.5.18-bin/bin:/nix/store/66v7hpgxzk49fby8zmcp4pri0m5z2agn-xwininfo-1.1.6/bin:/nix/store/vhsirn9m1ifmnw5g1qczzhvqkx6lw1if-findutils-4.10.0/bin:/nix/store/hx084k7pgz4n0vgkvil9gbcnl8y6p1xf-diffutils-3.12/bin:/nix/store/af4a8i43kc2ss4rnmf0swkk2mprsw6xq-gnused-4.9/bin:/nix/store/wf7lr2hf43546jc5kwqh3dbxnpcnw1mn-gnugrep-3.12/bin:/nix/store/lakv43kv98sl6h0ba6wnyg513mcq61vl-gawk-5.4.0/bin:/nix/store/rnvb7bvp53v2dw7pcwh9xb89x5z4rjib-gnutar-1.35/bin:/nix/store/9lhr1c3l9qzv8pzp3idmii1nwvxxjys3-gzip-1.14/bin:/nix/store/yvrwcs1a45rj8142n0l2w9q9s6akamjr-gnumake-4.4.1/bin:/nix/store/i27rhb3nr65rkrwz36bchkwmav6ggsmn-bash-5.3p9/bin:/nix/store/zj7mxwji29zvj9vl70iip7gw4h6ljfam-patch-2.8/bin:/nix/store/2nm5c858fh52s6mhcffm07s3biaxys44-xz-5.8.3-bin/bin:/nix/store/iscmg3ivhx7z67dz14lrg7p77gnsa4dw-file-5.45/bin:/home/exec/.mimocode/bin:/home/exec/.local/bin:/home/exec/.npm-global/bin:/home/exec/go/bin:/run/wrappers/bin:/guix/current/bin:/home/exec/.guix-home/profile/bin:/home/exec/.guix-profile/bin:/home/exec/.local/share/flatpak/exports/bin:/var/lib/flatpak/exports/bin:/home/exec/.nix-profile/bin:/nix/profile/bin:/home/exec/.local/state/nix/profile/bin:/etc/profiles/per-user/exec/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin:/home/exec/.zsh/plugins/fzf-tab:/home/exec/.zsh/plugins/zsh-nix-shell:/home/exec/.claude/plugins/cache/claude-plugins-official/superpowers/6.1.1/bin:/home/exec/.claude/plugins/cache/thedotmack/claude-mem/12.1.3/bin:/home/exec/.claude/plugins/cache/zai-coding-plugins/glm-plan-usage/0.0.1/bin\" nil t nil)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(list
  (getenv "HOME")
  (getenv "PATH")
  (getenv "NONEXISTENT_VAR_12345")
  (stringp (getenv "HOME"))
  (booleanp (getenv "HOME")))"#,
        expect,
    );
}
