use proguard::{ProguardMapper, StackFrame};

#[test]
fn test_remap() {
    // https://github.com/getsentry/rust-proguard/issues/5#issue-410310382
    let mapper = ProguardMapper::from(
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
        "    at some.Class.method3(Class.java:79)
    at some.Class.method2(Class.java:87)
    at some.Class.method1(Class.java:95)
    at some.Class.main(Class.java:101)\n"
    );

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605610326
    let mapper = ProguardMapper::from(
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
    let mapper = ProguardMapper::from(
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
fn test_remap_no_lines() {
    let mapper = ProguardMapper::from(
        r#"original.class.name -> a:
    void originalMethodName() -> b"#,
    );

    let mapped = mapper.remap_class("a");
    assert_eq!(mapped, Some("original.class.name"));

    let mut mapped = mapper.remap_frame(&StackFrame::new("a", "b", 10));
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::with_file("original.class.name", "originalMethodName", 0, "name.java")
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_remap_kotlin() {
    let mapper = ProguardMapper::from(
        r#"io.sentry.sample.-$$Lambda$r3Avcbztes2hicEObh02jjhQqd4 -> e.a.c.a:
    io.sentry.sample.MainActivity f$0 -> b
    1:1:void io.sentry.sample.KotlinSampleKt.fun3(io.sentry.sample.KotlinSample):16:16 -> onClick
    1:1:void io.sentry.sample.KotlinSample.fun2():11 -> onClick
    1:1:void io.sentry.sample.KotlinSample.fun1():7 -> onClick
    1:1:void io.sentry.sample.MainActivity.bar():56 -> onClick
    1:1:void io.sentry.sample.MainActivity.foo():44 -> onClick
    1:1:void io.sentry.sample.MainActivity.onClickHandler(android.view.View):40 -> onClick
    1:1:void onClick(android.view.View):0 -> onClick"#,
    );

    let mapped = mapper
        .remap_stacktrace("    at e.a.c.a.onClick(lambda:1)")
        .unwrap();

    assert_eq!(
        mapped.trim(),
        r#"at io.sentry.sample.KotlinSampleKt.fun3(KotlinSampleKt.java:16)
    at io.sentry.sample.KotlinSample.fun2(KotlinSample.java:11)
    at io.sentry.sample.KotlinSample.fun1(KotlinSample.java:7)
    at io.sentry.sample.MainActivity.bar(MainActivity.java:56)
    at io.sentry.sample.MainActivity.foo(MainActivity.java:44)
    at io.sentry.sample.MainActivity.onClickHandler(MainActivity.java:40)
    at io.sentry.sample.-$$Lambda$r3Avcbztes2hicEObh02jjhQqd4.onClick(-.java:0)"#
            .trim()
    );
}

#[test]
fn test_remap_just_method() {
    let mapper = ProguardMapper::from(
        r#"com.exmaple.app.MainActivity -> a.b.c.d:
    com.example1.domain.MyBean myBean -> p
    1:1:void <init>():11:11 -> <init>
    1:1:void buttonClicked(android.view.View):29:29 -> buttonClicked
    2:2:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked
    2:2:void buttonClicked(android.view.View):29 -> buttonClicked
    1:1:void onCreate(android.os.Bundle):17:17 -> onCreate
    2:5:void onCreate(android.os.Bundle):22:25 -> onCreate"#,
    );

    let unambiguous = mapper.remap_method("a.b.c.d", "onCreate");
    assert_eq!(
        unambiguous,
        Some(("com.exmaple.app.MainActivity", "onCreate"))
    );

    let ambiguous = mapper.remap_method("a.b.c.d", "buttonClicked");
    assert_eq!(ambiguous, None);
}
