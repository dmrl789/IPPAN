package org.ippan.wallet.data

import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import java.time.Instant
import kotlin.random.Random

interface WalletRepository {
    fun snapshot(): Flow<WalletSnapshot>
    suspend fun refresh()
    suspend fun submitTransfer(request: TransferRequest)
}

data class TransferRequest(
    val toAddress: String,
    val amount: Double,
    val symbol: String,
    val note: String? = null
)

/**
 * In-memory repository that mimics the responses of the actual IPPAN gateway.
 * It lets designers and Android engineers iterate on the wallet UI without
 * waiting for the chain to be reachable from the emulator.
 */
class FakeWalletRepository : WalletRepository {

    private val state = MutableStateFlow(seedData())

    override fun snapshot(): Flow<WalletSnapshot> = state.asStateFlow()

    override suspend fun refresh() {
        // Pretend we hit a remote endpoint.
        delay(600)
        state.update { current ->
            current.copy(
                totalBalance = current.tokens.sumOf { it.fiatValue },
                lastSync = Instant.now()
            )
        }
    }

    override suspend fun submitTransfer(request: TransferRequest) {
        delay(800)
        state.update { current ->
            val amount = request.amount.coerceAtLeast(0.0)
            val symbol = request.symbol.uppercase()
            val transactions = current.transactions.toMutableList().apply {
                add(0, WalletTransaction(
                    id = Random.nextInt(10_000, 99_999).toString(),
                    type = TransactionType.SEND,
                    amount = amount,
                    symbol = symbol,
                    counterparty = request.toAddress,
                    timestamp = Instant.now(),
                    status = TransactionStatus.PENDING
                ))
            }
            val tokens = current.tokens.map { token ->
                if (token.symbol.equals(symbol, ignoreCase = true)) {
                    token.copy(
                        balance = (token.balance - amount).coerceAtLeast(0.0),
                        fiatValue = (token.fiatValue - amount * estimatePrice(token.symbol)).coerceAtLeast(0.0)
                    )
                } else token
            }
            current.copy(
                tokens = tokens,
                transactions = transactions.toList(),
                totalBalance = tokens.sumOf { it.fiatValue },
                lastSync = Instant.now()
            )
        }
    }

    private fun seedData(): WalletSnapshot {
        val tokens = listOf(
            TokenBalance(symbol = "IPP", name = "IPPAN", balance = 1250.42, fiatValue = 1250.42),
            TokenBalance(symbol = "USDc", name = "IPPAN Dollar", balance = 480.00, fiatValue = 480.0),
            TokenBalance(symbol = "NDR", name = "Node Runner", balance = 98.33, fiatValue = 245.83)
        )
        val transactions = listOf(
            WalletTransaction(
                id = "98345",
                type = TransactionType.RECEIVE,
                amount = 250.0,
                symbol = "IPP",
                counterparty = "init.faucet",
                timestamp = Instant.now().minusSeconds(86_400),
                status = TransactionStatus.CONFIRMED
            ),
            WalletTransaction(
                id = "98344",
                type = TransactionType.SEND,
                amount = 32.0,
                symbol = "NDR",
                counterparty = "validator.16",
                timestamp = Instant.now().minusSeconds(172_800),
                status = TransactionStatus.CONFIRMED
            ),
            WalletTransaction(
                id = "98343",
                type = TransactionType.RECEIVE,
                amount = 100.0,
                symbol = "USDc",
                counterparty = "otc.partner",
                timestamp = Instant.now().minusSeconds(259_200),
                status = TransactionStatus.CONFIRMED
            )
        )
        return WalletSnapshot(
            accountAddress = "ippan1q8dlf4u9c0demoaddress",
            totalBalance = tokens.sumOf { it.fiatValue },
            fiatCurrency = "USD",
            tokens = tokens,
            transactions = transactions,
            lastSync = Instant.now()
        )
    }

    private fun estimatePrice(symbol: String): Double = when (symbol.uppercase()) {
        "IPP" -> 1.0
        "USDC" -> 1.0
        "NDR" -> 2.5
        else -> 1.0
    }
}
