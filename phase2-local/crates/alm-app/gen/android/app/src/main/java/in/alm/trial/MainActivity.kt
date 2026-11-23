package `in`.alm.trial

import android.content.Intent
import android.os.Bundle
import android.print.PrintAttributes
import android.print.PrintManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  // Android's WebView has no window.print(); expose the platform print and share dialogs.
  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(PlatformBridge(this, webView), "AlmAndroid")
  }
}

class PlatformBridge(private val activity: MainActivity, private val webView: WebView) {
  /** Opens the system print dialog for the current page; it offers printers and Save as PDF. */
  @JavascriptInterface
  fun print(title: String) {
    activity.runOnUiThread {
      val manager = activity.getSystemService(PrintManager::class.java)
      val name = title.ifBlank { "ALM document" }
      manager.print(
        name,
        webView.createPrintDocumentAdapter(name),
        PrintAttributes.Builder().setMediaSize(PrintAttributes.MediaSize.ISO_A4).build(),
      )
    }
  }

  /** Opens the system share sheet with plain text. */
  @JavascriptInterface
  fun share(text: String) {
    activity.runOnUiThread {
      val send = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(Intent.EXTRA_TEXT, text)
      }
      activity.startActivity(Intent.createChooser(send, "Share invoice"))
    }
  }
}
