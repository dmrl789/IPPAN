package org.ippan.wallet.data

import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import org.ippan.wallet.crypto.CryptoUtils
import org.ippan.wallet.network.IppanApiClient
import org.ippan.wallet.network.TransactionResponse
import org.ippan.wallet.security.SecureKeyStorage
import java.security.KeyPair
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/**
 * Production wallet repository that integrates with real IPPAN blockchain
 */
class ProductionWalletRepository(
    private val apiClient: IppanApiClient,
    private val secureStorage: SecureKeyStorage,
    private val cryptoUtils: CryptoUtils
) : WalletRepository {
    
    private val state = MutableStateFlow(createInitialState())
    
    private fun createInitialState(): WalletSnapshot {
        val address = secureStorage.getWalletAddress() ?: "Not initialized"
        return WalletSnapshot(
            accountAddress = address,
            totalBalance = 0.0,
            fiatCurrency = "USD",
            tokens = emptyList(),
            transactions = emptyList(),
            lastSync = Instant.now()
        )
    }
    
    override fun snapshot(): Flow<WalletSnapshot> = state.asStateFlow()
    
    override suspend fun refresh() {
        if (!secureStorage.hasWallet()) {
            state.update { currentState ->
                currentState.copy(
                    accountAddress = "No wallet found",
                    totalBalance = 0.0,
                    tokens = emptyList(),
                    transactions = emptyList()
                )
            }
            return
        }
        
        try {
            val address = secureStorage.getWalletAddress()!!
            
            // Fetch balance
            val balanceResult = apiClient.getBalance(address)
            if (balanceResult.isFailure) {
                throw Exception("Failed to fetch balance: ${balanceResult.exceptionOrNull()?.message}")
            }
            
            val balanceResponse = balanceResult.getOrThrow()
            
            // Fetch transactions
            val transactionsResult = apiClient.getTransactions(address, 50)
            if (transactionsResult.isFailure) {
                throw Exception("Failed to fetch transactions: ${transactionsResult.exceptionOrNull()?.message}")
            }
            
            val transactionResponses = transactionsResult.getOrThrow()
            
            // Convert API responses to domain models
            val tokens = listOf(
                TokenBalance(
                    symbol = balanceResponse.currency,
                    name = "IPPAN Token",
                    balance = balanceResponse.balance,
                    fiatValue = balanceResponse.balance * 0.1 // Mock fiat conversion
                )
            )
            
            val transactions = transactionResponses.map { response ->
                WalletTransaction(
                    id = response.id,
                    type = if (response.from == address) TransactionType.SEND else TransactionType.RECEIVE,
                    amount = response.amount,
                    symbol = response.currency,
                    counterparty = if (response.from == address) response.to else response.from,
                    timestamp = Instant.parse(response.timestamp),
                    status = if (response.status == "confirmed") TransactionStatus.CONFIRMED else TransactionStatus.PENDING
                )
            }
            
            state.update { currentState ->
                currentState.copy(
                    accountAddress = address,
                    totalBalance = balanceResponse.balance,
                    fiatCurrency = "USD",
                    tokens = tokens,
                    transactions = transactions,
                    lastSync = Instant.now()
                )
            }
            
        } catch (e: Exception) {
            // Update state with error information
            state.update { currentState ->
                currentState.copy(
                    accountAddress = "Error: ${e.message}",
                    totalBalance = 0.0,
                    tokens = emptyList(),
                    transactions = emptyList()
                )
            }
        }
    }
    
    override suspend fun submitTransfer(request: TransferRequest) {
        if (!secureStorage.hasWallet()) {
            throw Exception("No wallet found. Please create a wallet first.")
        }
        
        try {
            val address = secureStorage.getWalletAddress()!!
            val publicKey = secureStorage.getPublicKey()!!
            val encryptedPrivateKey = secureStorage.getEncryptedPrivateKey()!!
            
            // Get current nonce (in production, this would come from the API)
            val nonce = cryptoUtils.generateNonce()
            
            // Get gas price
            val gasPriceResult = apiClient.getGasPrice()
            if (gasPriceResult.isFailure) {
                throw Exception("Failed to get gas price: ${gasPriceResult.exceptionOrNull()?.message}")
            }
            
            val gasPrice = gasPriceResult.getOrThrow().gasPrice
            val gasLimit = 21000L // Standard gas limit for simple transfers
            
            // Create transaction hash for signing
            val transactionHash = cryptoUtils.createTransactionHash(
                from = address,
                to = request.toAddress,
                amount = request.amount,
                currency = request.symbol,
                nonce = nonce,
                gasPrice = gasPrice,
                gasLimit = gasLimit
            )
            
            // TODO: Decrypt private key and sign transaction
            // This would require the user's password/biometric authentication
            val signature = "mock_signature" // In production, this would be the actual signature
            
            // Create signed transaction request
            val signedTransaction = org.ippan.wallet.network.SignedTransactionRequest(
                from = address,
                to = request.toAddress,
                amount = request.amount,
                currency = request.symbol,
                nonce = nonce,
                gasPrice = gasPrice,
                gasLimit = gasLimit,
                signature = signature,
                publicKey = publicKey
            )
            
            // Submit transaction to blockchain
            val submissionResult = apiClient.submitTransaction(signedTransaction)
            if (submissionResult.isFailure) {
                throw Exception("Failed to submit transaction: ${submissionResult.exceptionOrNull()?.message}")
            }
            
            val submissionResponse = submissionResult.getOrThrow()
            if (submissionResponse.status != "success") {
                throw Exception("Transaction submission failed: ${submissionResponse.message}")
            }
            
            // Refresh wallet state after successful submission
            delay(2000) // Wait a bit for transaction to be processed
            refresh()
            
        } catch (e: Exception) {
            throw Exception("Transfer failed: ${e.message}")
        }
    }
    
    /**
     * Initialize a new wallet
     */
    suspend fun initializeWallet(): String {
        try {
            // Generate new key pair
            val keyPair = cryptoUtils.generateKeyPair()
            
            // Generate wallet address
            val address = cryptoUtils.generateAddress(keyPair.public)
            
            // Store wallet data securely
            secureStorage.storeWalletAddress(address)
            secureStorage.storePublicKey(Base64.encodeToString(keyPair.public.encoded, Base64.NO_WRAP))
            secureStorage.storeEncryptedPrivateKey(Base64.encodeToString(keyPair.private.encoded, Base64.NO_WRAP))
            secureStorage.setWalletCreated(true)
            
            // Refresh state
            refresh()
            
            return address
            
        } catch (e: Exception) {
            throw Exception("Failed to initialize wallet: ${e.message}")
        }
    }
    
    /**
     * Check if wallet is initialized
     */
    fun isWalletInitialized(): Boolean {
        return secureStorage.hasWallet()
    }
    
    /**
     * Get network status
     */
    suspend fun getNetworkStatus(): Result<org.ippan.wallet.network.NetworkStatusResponse> {
        return apiClient.getNetworkStatus()
    }
}
