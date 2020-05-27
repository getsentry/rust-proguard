use proguard::{Mapper, StackFrame};

#[test]
fn test_retrace() {
    // https://github.com/getsentry/rust-proguard/issues/5#issue-410310382
    let mapper = Mapper::new(
        r#"some.Class -> obfuscated:
    7:8:void method3(long):78:79 -> main
    7:8:void method2(int):87 -> main
    7:8:void method1(java.lang.String):95 -> main
    7:8:void main(java.lang.String[]):101 -> main"#,
    );
    let stacktrace = "    at obfuscated.main(Foo.java:8)";
    let mapped = mapper.remap_stacktrace(stacktrace).unwrap();
    assert_eq!(
        mapped,
        "    at some.Class.method3(Foo.java:79)
    at some.Class.method2(Foo.java:87)
    at some.Class.method1(Foo.java:95)
    at some.Class.main(Foo.java:101)\n"
    );

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605610326
    let mapper = Mapper::new(
        r#"com.exmaple.app.MainActivity -> com.exmaple.app.MainActivity:
    com.example1.domain.MyBean myBean -> p
    1:1:void <init>():11:11 -> <init>
    1:1:void buttonClicked(android.view.View):29:29 -> buttonClicked
    2:2:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked
    2:2:void buttonClicked(android.view.View):29 -> buttonClicked
    1:1:void onCreate(android.os.Bundle):17:17 -> onCreate
    2:5:void onCreate(android.os.Bundle):22:25 -> onCreate"#,
    );
    let stacktrace = "    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:2)";
    let mapped = mapper.remap_stacktrace(stacktrace).unwrap();
    assert_eq!(
        mapped,
        "    at com.example1.domain.MyBean.doWork(MyBean.java:16)
    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:29)\n"
    );

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605613412
    let mapper = Mapper::new(
        r#"com.exmaple.app.MainActivity -> com.exmaple.app.MainActivity:
    com.example1.domain.MyBean myBean -> k
    11:11:void <init>() -> <init>
    17:26:void onCreate(android.os.Bundle) -> onCreate
    29:30:void buttonClicked(android.view.View) -> buttonClicked
    1016:1016:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked
    1016:1016:void buttonClicked(android.view.View):29 -> buttonClicked"#,
    );
    let stacktrace = "    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:1016)";
    let mapped = mapper.remap_stacktrace(stacktrace).unwrap();
    assert_eq!(
        mapped,
        "    at com.example1.domain.MyBean.doWork(MyBean.java:16)
    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:29)\n"
    );
}

#[test]
fn test_retrace_synthetic() {
    let mapper = Mapper::new(
        r#"original.class.name -> a:
    1:1:void originalMethodName():10 -> b"#,
    );

    let frame = StackFrame::new("a", "does_not_exist", "", 0);
    let expected = StackFrame::new("original.class.name", "does_not_exist", "", 0);
    let mut actual = mapper.remap_frame(&frame);

    assert_eq!(actual.next().unwrap(), expected);
    assert_eq!(actual.next(), None);

    let frame = StackFrame::new("a", "b", "", 0);
    let expected = StackFrame::new("original.class.name", "originalMethodName", "", 0);
    let mut actual = mapper.remap_frame(&frame);

    assert_eq!(actual.next().unwrap(), expected);
    assert_eq!(actual.next(), None);
}

#[test]
fn test_retrace_member() {
    let mapper = Mapper::new(
        r#"
io.sentry.sample.-$$Lambda$MainActivity$4aaSlfCQgj_1XC8PygMYIPFzvbU -> d.a.c.a:
    io.sentry.sample.-$$Lambda$MainActivity$4aaSlfCQgj_1XC8PygMYIPFzvbU INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$C6WLFtOOCwWHrHZymygVPybkZJc -> d.a.c.b:
    io.sentry.sample.-$$Lambda$MainActivity$C6WLFtOOCwWHrHZymygVPybkZJc INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$Jj2f10fH_m5W5SRFf0Nj9WGDNDs -> d.a.c.c:
    io.sentry.sample.-$$Lambda$MainActivity$Jj2f10fH_m5W5SRFf0Nj9WGDNDs INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$OnutgJyvTK8aOxb9yV8WMSnn8y4 -> d.a.c.d:
    io.sentry.sample.-$$Lambda$MainActivity$OnutgJyvTK8aOxb9yV8WMSnn8y4 INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$W5V1SZYiQFLzkOXTNL5EFGi0luw -> d.a.c.e:
    io.sentry.sample.-$$Lambda$MainActivity$W5V1SZYiQFLzkOXTNL5EFGi0luw INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$lx6wy8pOXx_tKjUileeSycxvo_Q -> d.a.c.f:
    io.sentry.sample.-$$Lambda$MainActivity$lx6wy8pOXx_tKjUileeSycxvo_Q INSTANCE -> b
io.sentry.sample.-$$Lambda$MainActivity$tVGPRGxxb8SivUa5SKhzp6BuXOI -> d.a.c.g:
    io.sentry.sample.-$$Lambda$MainActivity$tVGPRGxxb8SivUa5SKhzp6BuXOI INSTANCE -> b
io.sentry.sample.MainActivity -> io.sentry.sample.MainActivity:
    void lambda$onCreate$0(android.view.View) -> a
    void lambda$onCreate$1(android.view.View) -> b
    void lambda$onCreate$2(android.view.View) -> c
    void lambda$onCreate$3(android.view.View) -> d
    void lambda$onCreate$4(android.view.View) -> e
    void lambda$onCreate$5(android.view.View) -> f
    void lambda$onCreate$6(android.view.View) -> g
    1:1:void timber.log.Timber.i(java.lang.String,java.lang.Object[]):0:0 -> onCreate
    1:1:void onCreate(android.os.Bundle):0 -> onCreate
    2:2:void onCreate(android.os.Bundle):0:0 -> onCreate"#,
    );

    let frame = StackFrame::new("io.sentry.sample.MainActivity", "c", "", 14);
    let expected = StackFrame::new("io.sentry.sample.MainActivity", "lambda$onCreate$2", "", 14);
    let mut actual = mapper.remap_frame(&frame);

    assert_eq!(actual.next().unwrap(), expected);
    assert_eq!(actual.next(), None);

    let frame = StackFrame::new("d.a.c.g", "onClick", "", 0);
    let expected = StackFrame::new(
        "io.sentry.sample.-$$Lambda$MainActivity$tVGPRGxxb8SivUa5SKhzp6BuXOI",
        "onClick",
        "",
        0,
    );
    let mut actual = mapper.remap_frame(&frame);

    assert_eq!(actual.next().unwrap(), expected);
    assert_eq!(actual.next(), None);
}
