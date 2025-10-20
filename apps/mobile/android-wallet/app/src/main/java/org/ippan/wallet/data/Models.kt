package org.ippan.wallet.data

import java.time.Instant

/** Token balance for a specific IPPAN asset. */
data class TokenBalance(
    val symbol: String,
    val name: String,
    val balance: Double,
    val fiatValue: Double
)

enum class TransactionType { SEND, RECEIVE }

enum class TransactionStatus { CONFIRMED, PENDING }

/** Representation of a wallet movement. */
data class WalletTransaction(
    val id: String,
    val type: TransactionType,
    val amount: Double,
    val symbol: String,
    val counterparty: String,
    val timestamp: Instant,
    val status: TransactionStatus
)

/** Aggregated snapshot returned by the repository. */
data class WalletSnapshot(
    val accountAddress: String,
    val totalBalance: Double,
    val fiatCurrency: String,
    val tokens: List<TokenBalance>,
    val transactions: List<WalletTransaction>,
    val lastSync: Instant,
    val activeNode: String
)

// TransferRequest lives in WalletRepository.kt to avoid duplicate declarations

/** Comprehensive error handling for wallet operations */
data class WalletError(
    val title: String,
    val message: String,
    val severity: Severity,
    val suggestion: String? = null,
    val isRetryable: Boolean = true,
    val errorCode: String? = null
) {
    enum class Severity {
        INFO, WARNING, ERROR
    }
    
    companion object {
        fun networkError(message: String): WalletError {
            return WalletError(
                title = "Network Error",
                message = message,
                severity = Severity.ERROR,
                suggestion = "Check your internet connection and try again",
                isRetryable = true,
                errorCode = "NETWORK_ERROR"
            )
        }
        
        fun authenticationError(message: String): WalletError {
            return WalletError(
                title = "Authentication Failed",
                message = message,
                severity = Severity.ERROR,
                suggestion = "Please try authenticating again",
                isRetryable = true,
                errorCode = "AUTH_ERROR"
            )
        }
        
        fun transactionError(message: String): WalletError {
            return WalletError(
                title = "Transaction Failed",
                message = message,
                severity = Severity.ERROR,
                suggestion = "Check the recipient address and try again",
                isRetryable = true,
                errorCode = "TX_ERROR"
            )
        }
        
        fun insufficientFundsError(required: Double, available: Double): WalletError {
            return WalletError(
                title = "Insufficient Funds",
                message = "You need ${required} tokens but only have ${available}",
                severity = Severity.ERROR,
                suggestion = "Add more tokens to your wallet or reduce the amount",
                isRetryable = false,
                errorCode = "INSUFFICIENT_FUNDS"
            )
        }
        
        fun invalidAddressError(address: String): WalletError {
            return WalletError(
                title = "Invalid Address",
                message = "The address '$address' is not valid",
                severity = Severity.ERROR,
                suggestion = "Please check the address and try again",
                isRetryable = false,
                errorCode = "INVALID_ADDRESS"
            )
        }
    }
}

// WalletRepository interface lives in WalletRepository.kt to avoid duplicates
