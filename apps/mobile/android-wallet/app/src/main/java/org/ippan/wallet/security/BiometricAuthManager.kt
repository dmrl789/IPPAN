package org.ippan.wallet.security

import android.content.Context
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Biometric authentication manager for secure wallet operations
 */
class BiometricAuthManager(private val context: Context) {
    
    private val biometricManager = BiometricManager.from(context)
    
    /**
     * Check if biometric authentication is available
     */
    fun isBiometricAvailable(): Boolean {
        return when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK)) {
            BiometricManager.BIOMETRIC_SUCCESS -> true
            else -> false
        }
    }
    
    /**
     * Authenticate user with biometrics
     */
    suspend fun authenticate(
        activity: FragmentActivity,
        title: String = "Authenticate",
        subtitle: String = "Use your biometric to access your wallet",
        negativeButtonText: String = "Cancel"
    ): Result<Unit> = suspendCancellableCoroutine { continuation ->
        
        val executor = ContextCompat.getMainExecutor(context)
        val biometricPrompt = BiometricPrompt(activity, executor, object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                continuation.resume(Result.success(Unit))
            }
            
            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                continuation.resumeWithException(
                    BiometricAuthException("Authentication error: $errString", errorCode)
                )
            }
            
            override fun onAuthenticationFailed() {
                continuation.resumeWithException(
                    BiometricAuthException("Authentication failed", -1)
                )
            }
        })
        
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setNegativeButtonText(negativeButtonText)
            .build()
        
        continuation.invokeOnCancellation {
            biometricPrompt.cancelAuthentication()
        }
        
        biometricPrompt.authenticate(promptInfo)
    }
    
    /**
     * Authenticate for transaction signing
     */
    suspend fun authenticateForTransaction(activity: FragmentActivity): Result<Unit> {
        return authenticate(
            activity = activity,
            title = "Sign Transaction",
            subtitle = "Use your biometric to sign this transaction",
            negativeButtonText = "Cancel"
        )
    }
    
    /**
     * Authenticate for wallet access
     */
    suspend fun authenticateForWalletAccess(activity: FragmentActivity): Result<Unit> {
        return authenticate(
            activity = activity,
            title = "Access Wallet",
            subtitle = "Use your biometric to access your wallet",
            negativeButtonText = "Cancel"
        )
    }
}

/**
 * Biometric authentication exception
 */
class BiometricAuthException(
    message: String,
    val errorCode: Int
) : Exception(message)
