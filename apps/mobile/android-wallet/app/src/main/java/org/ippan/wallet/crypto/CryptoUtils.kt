package org.ippan.wallet.crypto

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.bouncycastle.jce.provider.BouncyCastleProvider
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.PrivateKey
import java.security.PublicKey
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.PBEKeySpec
import javax.crypto.spec.SecretKeySpec
import kotlin.random.Random

/**
 * Cryptographic utilities for IPPAN wallet operations
 */
object CryptoUtils {

    private const val KEYSTORE_ALIAS = "ippan_wallet_key"
    private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    private const val SIGNATURE_ALGORITHM = "SHA256withECDSA"
    private const val ENCRYPTION_ALGORITHM = "AES/GCM/NoPadding"
    private const val GCM_IV_LENGTH = 12
    private const val GCM_TAG_LENGTH = 16

    init {
        if (java.security.Security.getProvider(BouncyCastleProvider.PROVIDER_NAME) == null) {
            java.security.Security.addProvider(BouncyCastleProvider())
        }
    }

    /**
     * Generate a new ECDSA key pair for wallet operations
     */
    fun generateKeyPair(alias: String = KEYSTORE_ALIAS): KeyPair {
        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_EC,
            ANDROID_KEYSTORE
        )
        val parameterSpec = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
        )
            .setDigests(KeyProperties.DIGEST_SHA256, KeyProperties.DIGEST_SHA512)
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            .build()

        generator.initialize(parameterSpec)
        return generator.generateKeyPair()
    }

    fun getPrivateKey(alias: String = KEYSTORE_ALIAS): PrivateKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val key = keyStore.getKey(alias, null)
            ?: throw IllegalStateException("No private key found for alias $alias")
        return key as PrivateKey
    }

    fun getPublicKey(alias: String = KEYSTORE_ALIAS): PublicKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        return keyStore.getCertificate(alias)?.publicKey
            ?: throw IllegalStateException("No public key certificate for alias $alias")
    }

    /**
     * Sign a transaction with the private key
     */
    fun signTransaction(transactionData: ByteArray, privateKey: PrivateKey): String {
        val signature = Signature.getInstance(SIGNATURE_ALGORITHM)
        signature.initSign(privateKey)
        signature.update(transactionData)
        val signatureBytes = signature.sign()
        return Base64.encodeToString(signatureBytes, Base64.NO_WRAP)
    }

    /**
     * Verify a transaction signature
     */
    fun verifySignature(transactionData: ByteArray, signature: String, publicKey: PublicKey): Boolean {
        return try {
            val signatureBytes = Base64.decode(signature, Base64.DEFAULT)
            val sig = Signature.getInstance(SIGNATURE_ALGORITHM)
            sig.initVerify(publicKey)
            sig.update(transactionData)
            sig.verify(signatureBytes)
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Generate a wallet address from public key
     */
    fun generateAddress(publicKey: PublicKey): String {
        val publicKeyBytes = publicKey.encoded
        val hash = hash160(publicKeyBytes)
        return "ippan_${Base64.encodeToString(hash, Base64.NO_WRAP)}"
    }

    /**
     * Encrypt sensitive data (like private keys) using AES-GCM
     */
    fun encryptData(data: ByteArray, password: String): EncryptedData {
        val key = generateAESKey(password)
        val cipher = Cipher.getInstance(ENCRYPTION_ALGORITHM)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        
        val iv = cipher.iv
        val encryptedData = cipher.doFinal(data)
        
        return EncryptedData(
            data = encryptedData,
            iv = iv,
            tag = encryptedData.takeLast(GCM_TAG_LENGTH).toByteArray()
        )
    }

    /**
     * Decrypt sensitive data
     */
    fun decryptData(encryptedData: EncryptedData, password: String): ByteArray {
        val key = generateAESKey(password)
        val cipher = Cipher.getInstance(ENCRYPTION_ALGORITHM)
        val spec = GCMParameterSpec(GCM_TAG_LENGTH * 8, encryptedData.iv)
        cipher.init(Cipher.DECRYPT_MODE, key, spec)
        
        return cipher.doFinal(encryptedData.data)
    }

    /**
     * Generate a secure random nonce for transactions
     */
    fun generateNonce(): Long {
        return Random.nextLong(0, Long.MAX_VALUE)
    }

    /**
     * Hash data using SHA-256
     */
    fun sha256(data: ByteArray): ByteArray {
        val digest = MessageDigest.getInstance("SHA-256")
        return digest.digest(data)
    }

    /**
     * Hash data using RIPEMD-160 (for address generation)
     */
    private fun hash160(data: ByteArray): ByteArray {
        val sha256 = MessageDigest.getInstance("SHA-256")
        val ripemd160 = MessageDigest.getInstance("RipeMD160", BouncyCastleProvider.PROVIDER_NAME)
        val sha256Hash = sha256.digest(data)
        return ripemd160.digest(sha256Hash)
    }

    /**
     * Generate AES key from password
     */
    private fun generateAESKey(password: String): SecretKey {
        val salt = password.toByteArray()
        val keySpec = PBEKeySpec(
            password.toCharArray(),
            salt,
            10000, // iterations
            256 // key length
        )
        val keyFactory = SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256")
        val key = keyFactory.generateSecret(keySpec)
        return SecretKeySpec(key.encoded, "AES")
    }

    /**
     * Validate wallet address format
     */
    fun isValidAddress(address: String): Boolean {
        return address.startsWith("ippan_") && address.length in 34..48
    }

    /**
     * Create transaction hash for signing
     */
    fun createTransactionHash(
        from: String,
        to: String,
        amount: Double,
        currency: String,
        nonce: Long,
        gasPrice: Long,
        gasLimit: Long
    ): ByteArray {
        val transactionString = "$from:$to:$amount:$currency:$nonce:$gasPrice:$gasLimit"
        return sha256(transactionString.toByteArray())
    }
}

/**
 * Encrypted data container
 */
data class EncryptedData(
    val data: ByteArray,
    val iv: ByteArray,
    val tag: ByteArray
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as EncryptedData

        if (!data.contentEquals(other.data)) return false
        if (!iv.contentEquals(other.iv)) return false
        if (!tag.contentEquals(other.tag)) return false

        return true
    }

    override fun hashCode(): Int {
        var result = data.contentHashCode()
        result = 31 * result + iv.contentHashCode()
        result = 31 * result + tag.contentHashCode()
        return result
    }
}
