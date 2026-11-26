package `in`.alm.trial

import android.app.Activity
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class SecretArgs {
  lateinit var account: String
  var token: String? = null
}

/**
 * The signed-in person's refresh token, called from Rust (credentials.rs): encrypted with an
 * AES-GCM key that the Android Keystore holds (it never leaves the Keystore) and stored as
 * IV + ciphertext in app-private storage, one file per account.
 */
@TauriPlugin
class SecretsPlugin(private val activity: Activity) : Plugin(activity) {
  private val alias = "alm-refresh-token"

  private fun file(account: String): File {
    val name = account.toByteArray().joinToString("") { "%02x".format(it) }
    return File(File(activity.filesDir, "credentials"), "$name.bin")
  }

  private fun key(): SecretKey {
    val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    (store.getEntry(alias, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
    val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
    generator.init(
      KeyGenParameterSpec.Builder(
        alias,
        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
      )
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setKeySize(256)
        .build(),
    )
    return generator.generateKey()
  }

  @Command
  fun save(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(SecretArgs::class.java)
      val cipher = Cipher.getInstance("AES/GCM/NoPadding")
      cipher.init(Cipher.ENCRYPT_MODE, key())
      val sealed = cipher.iv + cipher.doFinal((args.token ?: "").toByteArray())
      val target = file(args.account)
      target.parentFile?.mkdirs()
      val staged = File(target.path + ".tmp")
      staged.writeBytes(sealed)
      if (!staged.renameTo(target)) throw IllegalStateException("rename failed")
      invoke.resolve(JSObject())
    } catch (error: Exception) {
      invoke.reject("credential save failed: ${error.javaClass.simpleName}")
    }
  }

  @Command
  fun load(invoke: Invoke) {
    val result = JSObject()
    try {
      val args = invoke.parseArgs(SecretArgs::class.java)
      val source = file(args.account)
      if (source.exists()) {
        val sealed = source.readBytes()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, sealed, 0, 12))
        result.put("token", String(cipher.doFinal(sealed, 12, sealed.size - 12)))
      }
      invoke.resolve(result)
    } catch (error: Exception) {
      invoke.reject("credential load failed: ${error.javaClass.simpleName}")
    }
  }

  @Command
  fun delete(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(SecretArgs::class.java)
      file(args.account).delete()
      invoke.resolve(JSObject())
    } catch (error: Exception) {
      invoke.reject("credential delete failed: ${error.javaClass.simpleName}")
    }
  }
}
