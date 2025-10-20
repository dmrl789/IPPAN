package org.ippan.wallet.data

import android.util.Base64
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import org.ippan.wallet.crypto.CryptoUtils
import org.ippan.wallet.network.IppanApiClient
import org.ippan.wallet.security.SecureKeyStorage
import java.time.Instant

/**
 * Production wallet repository that integrates with real IPPAN blockchain
 */
class ProductionWalletRepository(
    private val apiClient: IppanApiClient,
    private val secureStorage: SecureKeyStorage,
    private val cryptoUtils: CryptoUtils,
    private val fiatConversionService: FiatConversionService = FiatConversionService()
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
            lastSync = Instant.now(),
            activeNode = apiClient.activeNode
        )
    }

    override fun snapshot(): Flow<WalletSnapshot> = state.asStateFlow()

    override suspend fun refresh() {
        val address = ensureWalletAddress()

        val balanceResponse = apiClient.getBalance(address).getOrElse { error ->
            throw IllegalStateException("Failed to fetch balance", error)
        }

        val transactionResponses = apiClient.getTransactions(address, 50).getOrElse { error ->
            throw IllegalStateException("Failed to fetch transactions", error)
        }

        // Get real fiat conversion rate
        val fiatRate = fiatConversionService.getExchangeRate(
            fromCurrency = balanceResponse.currency,
            toCurrency = "USD"
        ).getOrElse { 
            // Fallback to placeholder if conversion fails
            ExchangeRate(
                from = balanceResponse.currency,
                to = "USD", 
                rate = FIAT_PLACEHOLDER_MULTIPLIER,
                timestamp = System.currentTimeMillis(),
                source = "fallback"
            )
        }

        val tokens = listOf(
            TokenBalance(
                symbol = balanceResponse.currency,
                name = "IPPAN Token",
                balance = balanceResponse.balance,
                fiatValue = balanceResponse.balance * fiatRate.rate
            )
        )

        val transactions = transactionResponses.map { response ->
            WalletTransaction(
                id = response.id,
                type = if (response.from.equals(address, ignoreCase = true)) {
                    TransactionType.SEND
                } else {
                    TransactionType.RECEIVE
                },
                amount = response.amount,
                symbol = response.currency,
                counterparty = if (response.from.equals(address, ignoreCase = true)) {
                    response.to
                } else {
                    response.from
                },
                timestamp = Instant.parse(response.timestamp),
                status = if (response.status.equals("confirmed", ignoreCase = true)) {
                    TransactionStatus.CONFIRMED
                } else {
                    TransactionStatus.PENDING
                }
            )
        }

        state.update {
            it.copy(
                accountAddress = address,
                totalBalance = balanceResponse.balance,
                fiatCurrency = balanceResponse.currency,
                tokens = tokens,
                transactions = transactions,
                lastSync = Instant.now(),
                activeNode = apiClient.activeNode
            )
        }
    }

    override suspend fun submitTransfer(request: TransferRequest) {
        if (!cryptoUtils.isValidAddress(request.toAddress)) {
            throw IllegalArgumentException("Destination address is not a valid IPPAN address")
        }

        try {
            val address = ensureWalletAddress()
            val publicKey = ensurePublicKey()
            val privateKeyAlias = secureStorage.getPrivateKeyAlias()
                ?: throw IllegalStateException("Missing private key alias")
            val privateKey = cryptoUtils.getPrivateKey(privateKeyAlias)

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

            // Sign transaction payload using hardware-backed key
            val signature = cryptoUtils.signTransaction(transactionHash, privateKey)

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
            throw IllegalStateException("Transfer failed", e)
        }
    }

    /**
     * Initialize a new wallet
     */
    suspend fun initializeWallet(): String {
        try {
            val alias = "ippan_wallet_${System.currentTimeMillis()}"
            val keyPair = cryptoUtils.generateKeyPair(alias)

            val publicKeyEncoded = Base64.encodeToString(keyPair.public.encoded, Base64.NO_WRAP)
            val address = cryptoUtils.generateAddress(keyPair.public)

            secureStorage.storeWalletAddress(address)
            secureStorage.storePublicKey(publicKeyEncoded)
            secureStorage.storePrivateKeyAlias(alias)
            secureStorage.setWalletCreated(true)

            state.update {
                it.copy(
                    accountAddress = address,
                    activeNode = apiClient.activeNode
                )
            }

            return address
        } catch (e: Exception) {
            throw IllegalStateException("Failed to initialize wallet", e)
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

    private fun ensurePublicKey(): String {
        val alias = secureStorage.getPrivateKeyAlias()
        val stored = secureStorage.getPublicKey()
        if (alias == null) {
            return stored ?: throw IllegalStateException("Wallet not initialized")
        }
        if (stored != null) {
            return stored
        }
        val publicKey = cryptoUtils.getPublicKey(alias)
        val encoded = Base64.encodeToString(publicKey.encoded, Base64.NO_WRAP)
        secureStorage.storePublicKey(encoded)
        return encoded
    }

    private suspend fun ensureWalletAddress(): String {
        if (!secureStorage.hasWallet()) {
            initializeWallet()
        }
        return secureStorage.getWalletAddress()
            ?: throw IllegalStateException("Wallet not initialized")
    }

    companion object {
        private const val FIAT_PLACEHOLDER_MULTIPLIER = 0.1
    }
}
