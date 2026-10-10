/* On-device JNI .so load smoke test (shell domain, no APK).
 * dlopen the bridge + dlsym the entry point. Calling it needs a JNIEnv,
 * which only exists inside an app process — out of scope here. */
#include <dlfcn.h>
#include <stdio.h>

int main(void) {
    void *h = dlopen("/data/local/tmp/librise_jni.so", RTLD_NOW);
    if (!h) {
        printf("DLOPEN_FAIL: %s\n", dlerror());
        return 1;
    }
    void *s = dlsym(h, "Java_com_rise_core_RiseCore_hashFrame");
    if (!s) {
        printf("DLSYM_FAIL: %s\n", dlerror());
        return 2;
    }
    printf("JNI_SYMBOL_OK\n");
    return 0;
}
