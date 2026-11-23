package `in`.alm.trial

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.print.PrintAttributes
import android.print.PrintManager
import android.view.View
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    @Suppress("DEPRECATION") // Request IME inset delivery to the edge-to-edge content.
    window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val safeArea = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
      )
      view.setPadding(safeArea.left, safeArea.top, safeArea.right, safeArea.bottom)
      // Pass the remaining insets (including IME) in the WebView's smaller viewport.
      insets.inset(safeArea.left, safeArea.top, safeArea.right, safeArea.bottom)
    }
    ViewCompat.requestApplyInsets(content)
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

  /** Shares a file the app wrote to its cache output folder (bill PNG, CSV or XLS export). */
  @JavascriptInterface
  fun shareFile(path: String) {
    activity.runOnUiThread {
      val file = java.io.File(path)
      if (!file.canonicalPath.startsWith(activity.cacheDir.canonicalPath)) return@runOnUiThread
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
      val send = Intent(Intent.ACTION_SEND).apply {
        type = when (file.extension) {
          "png" -> "image/png"
          "csv" -> "text/csv"
          else -> "application/vnd.ms-excel"
        }
        putExtra(Intent.EXTRA_STREAM, uri)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      }
      activity.startActivity(Intent.createChooser(send, "Share ${file.name}"))
    }
  }

  /** Opens a WhatsApp link (wa.me only) in WhatsApp or the browser. */
  @JavascriptInterface
  fun openUrl(url: String) {
    if (!url.startsWith("https://wa.me/")) return
    activity.runOnUiThread {
      activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    }
  }
}
