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
