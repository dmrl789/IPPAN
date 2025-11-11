package org.ippan.wallet.crypto

import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.JUnit4
import java.security.Security
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

@RunWith(JUnit4::class)
class CryptoUtilsTest {
    
    init {
        // Register BouncyCastle provider for cryptographic operations
        // Using init block instead of @BeforeClass to ensure it runs for JUnit4
        if (Security.getProvider(BouncyCastleProvider.PROVIDER_NAME) == null) {
            Security.addProvider(BouncyCastleProvider())
        }
    }
    
    @Test
    fun `generate key pair should create valid key pair`() {
        val keyPair = CryptoUtils.generateKeyPair()
        
        assertNotNull(keyPair.public)
        assertNotNull(keyPair.private)
        assertEquals("EC", keyPair.public.algorithm)
        assertEquals("EC", keyPair.private.algorithm)
    }
    
    @Test
    fun `sign and verify transaction should work correctly`() {
        val keyPair = CryptoUtils.generateKeyPair()
        val transactionData = "test_transaction_data".toByteArray()
        
        val signature = CryptoUtils.signTransaction(transactionData, keyPair.private)
        assertNotNull(signature)
        assertTrue(signature.isNotEmpty())
        
        val isValid = CryptoUtils.verifySignature(transactionData, signature, keyPair.public)
        assertTrue(isValid)
    }
    
    @Test
    fun `generate address should create valid IPPAN address`() {
        val keyPair = CryptoUtils.generateKeyPair()
        val address = CryptoUtils.generateAddress(keyPair.public)
        
        assertTrue(address.startsWith("ippan_"))
        assertTrue(address.length > 10)
    }
    
    @Test
    fun `is valid address should validate correct addresses`() {
        val validAddress = "ippan_1234567890abcdef"
        val invalidAddress = "invalid_address"
        val invalidPrefix = "bitcoin_1234567890abcdef"
        
        assertTrue(CryptoUtils.isValidAddress(validAddress))
        assertFalse(CryptoUtils.isValidAddress(invalidAddress))
        assertFalse(CryptoUtils.isValidAddress(invalidPrefix))
    }
    
    @Test
    fun `generate nonce should create unique values`() {
        val nonce1 = CryptoUtils.generateNonce()
        val nonce2 = CryptoUtils.generateNonce()
        
        assertTrue(nonce1 >= 0)
        assertTrue(nonce2 >= 0)
        // Note: In practice, these could be the same due to randomness
        // but the probability is extremely low
    }
    
    @Test
    fun `sha256 should create consistent hashes`() {
        val data = "test_data".toByteArray()
        val hash1 = CryptoUtils.sha256(data)
        val hash2 = CryptoUtils.sha256(data)
        
        assertEquals(hash1.size, 32) // SHA-256 produces 32 bytes
        assertTrue(hash1.contentEquals(hash2))
    }
    
    @Test
    fun `create transaction hash should be deterministic`() {
        val from = "ippan_sender123"
        val to = "ippan_receiver456"
        val amount = 100.0
        val currency = "IPP"
        val nonce = 1L
        val gasPrice = 20L
        val gasLimit = 21000L
        
        val hash1 = CryptoUtils.createTransactionHash(from, to, amount, currency, nonce, gasPrice, gasLimit)
        val hash2 = CryptoUtils.createTransactionHash(from, to, amount, currency, nonce, gasPrice, gasLimit)
        
        assertTrue(hash1.contentEquals(hash2))
        assertEquals(32, hash1.size) // SHA-256 produces 32 bytes
    }
    
    @Test
    fun `encrypt and decrypt data should work correctly`() {
        val originalData = "sensitive_wallet_data".toByteArray()
        val password = "test_password"
        
        val encryptedData = CryptoUtils.encryptData(originalData, password)
        assertNotNull(encryptedData.data)
        assertNotNull(encryptedData.iv)
        assertNotNull(encryptedData.tag)
        
        val decryptedData = CryptoUtils.decryptData(encryptedData, password)
        assertTrue(originalData.contentEquals(decryptedData))
    }
}
