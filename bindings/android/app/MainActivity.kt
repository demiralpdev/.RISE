package rise.android.app

import android.app.Activity
import android.os.Bundle
import android.util.Log
import com.rise.core.RiseCore

/** Minimal app: proves System.loadLibrary("rise_jni") + hashFrame work in a
 *  real Android app process. Logs HASH=<64 hex> under the RISE tag. */
class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val proof = "rise-jni-apk-proof-001".toByteArray()
        val hex = RiseCore.hashFrame(proof)
        Log.i("RISE", "HASH=$hex")
    }
}
