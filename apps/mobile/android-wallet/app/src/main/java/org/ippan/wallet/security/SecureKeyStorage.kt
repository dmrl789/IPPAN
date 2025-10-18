package org.ippan.wallet.security

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey

/**
 * Secure storage for wallet private keys and sensitive data
 */
class SecureKeyStorage(private val context: Context) {
    
    private val masterKey = MasterKey.Builder(context)
        .setKeyGenParameterSpec(
            KeyGenParameterSpec.Builder(
                MasterKey.DEFAULT_MASTER_KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        .build()
    
    private val encryptedPrefs: SharedPreferences = EncryptedSharedPreferences.create(
        context,
        "ippan_wallet_secure",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
    
    companion object {
        private const val KEY_WALLET_ADDRESS = "wallet_address"
        private const val KEY_PUBLIC_KEY = "public_key"
        private const val KEY_ENCRYPTED_PRIVATE_KEY = "encrypted_private_key"
        private const val KEY_WALLET_CREATED = "wallet_created"
        private const val KEY_LAST_BACKUP = "last_backup"
        private const val KEY_BIOMETRIC_ENABLED = "biometric_enabled"
    }
    
    /**
     * Store wallet address securely
     */
    fun storeWalletAddress(address: String) {
        encryptedPrefs.edit()
            .putString(KEY_WALLET_ADDRESS, address)
            .apply()
    }
    
    /**
     * Get stored wallet address
     */
    fun getWalletAddress(): String? {
        return encryptedPrefs.getString(KEY_WALLET_ADDRESS, null)
    }
    
    /**
     * Store public key
     */
    fun storePublicKey(publicKey: String) {
        encryptedPrefs.edit()
            .putString(KEY_PUBLIC_KEY, publicKey)
            .apply()
    }
    
    /**
     * Get stored public key
     */
    fun getPublicKey(): String? {
        return encryptedPrefs.getString(KEY_PUBLIC_KEY, null)
    }
    
    /**
     * Store encrypted private key
     */
    fun storeEncryptedPrivateKey(encryptedPrivateKey: String) {
        encryptedPrefs.edit()
            .putString(KEY_ENCRYPTED_PRIVATE_KEY, encryptedPrivateKey)
            .apply()
    }
    
    /**
     * Get encrypted private key
     */
    fun getEncryptedPrivateKey(): String? {
        return encryptedPrefs.getString(KEY_ENCRYPTED_PRIVATE_KEY, null)
    }
    
    /**
     * Mark wallet as created
     */
    fun setWalletCreated(created: Boolean) {
        encryptedPrefs.edit()
            .putBoolean(KEY_WALLET_CREATED, created)
            .apply()
    }
    
    /**
     * Check if wallet is created
     */
    fun isWalletCreated(): Boolean {
        return encryptedPrefs.getBoolean(KEY_WALLET_CREATED, false)
    }
    
    /**
     * Set last backup timestamp
     */
    fun setLastBackup(timestamp: Long) {
        encryptedPrefs.edit()
            .putLong(KEY_LAST_BACKUP, timestamp)
            .apply()
    }
    
    /**
     * Get last backup timestamp
     */
    fun getLastBackup(): Long {
        return encryptedPrefs.getLong(KEY_LAST_BACKUP, 0L)
    }
    
    /**
     * Enable/disable biometric authentication
     */
    fun setBiometricEnabled(enabled: Boolean) {
        encryptedPrefs.edit()
            .putBoolean(KEY_BIOMETRIC_ENABLED, enabled)
            .apply()
    }
    
    /**
     * Check if biometric authentication is enabled
     */
    fun isBiometricEnabled(): Boolean {
        return encryptedPrefs.getBoolean(KEY_BIOMETRIC_ENABLED, false)
    }
    
    /**
     * Clear all stored data (for wallet reset)
     */
    fun clearAllData() {
        encryptedPrefs.edit().clear().apply()
    }
    
    /**
     * Check if wallet exists
     */
    fun hasWallet(): Boolean {
        return isWalletCreated() && getWalletAddress() != null && getPublicKey() != null
    }
    
    /**
     * Get wallet info for backup
     */
    fun getWalletInfo(): WalletInfo? {
        val address = getWalletAddress() ?: return null
        val publicKey = getPublicKey() ?: return null
        val created = encryptedPrefs.getLong(KEY_WALLET_CREATED, 0L)
        
        return WalletInfo(
            address = address,
            publicKey = publicKey,
            created = created,
            lastBackup = getLastBackup()
        )
    }
}

/**
 * Wallet information for backup purposes
 */
data class WalletInfo(
    val address: String,
    val publicKey: String,
    val created: Long,
    val lastBackup: Long
)
