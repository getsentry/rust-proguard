package io.sentry.samples.instrumentation.ui

class TestSourceContext {

    fun test() {
        test2()
    }

    fun test2() {
        throw IllegalStateException("checking line numbers in source context")
    }
}
