use proguard::{Mapper, StackFrame};

#[test]
fn test_remap() {
    // https://github.com/getsentry/rust-proguard/issues/5#issue-410310382
    let mapper = Mapper::from(
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
    let mapper = Mapper::from(
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
        "    at com.example1.domain.MyBean.doWork(<unknown>:16)
    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:29)\n"
    );

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605613412
    let mapper = Mapper::from(
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
        "    at com.example1.domain.MyBean.doWork(<unknown>:16)
    at com.exmaple.app.MainActivity.buttonClicked(MainActivity.java:29)\n"
    );
}

#[test]
fn test_remap_no_lines() {
    let mapper = Mapper::from(
        r#"original.class.name -> a:
    void originalMethodName() -> b"#,
    );

    let mapped = mapper.remap_class("a");
    assert_eq!(mapped, Some("original.class.name"));

    let mut mapped = mapper.remap_frame(&StackFrame::new("a", "b", 10));
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("original.class.name", "originalMethodName", 0)
    );
    assert_eq!(mapped.next(), None);
}
