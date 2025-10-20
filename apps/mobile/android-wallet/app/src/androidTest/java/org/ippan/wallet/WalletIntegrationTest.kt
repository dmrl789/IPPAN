package org.ippan.wallet

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.ippan.wallet.data.FakeWalletRepository
import org.ippan.wallet.data.WalletSnapshot
import org.ippan.wallet.data.TokenBalance
import org.ippan.wallet.data.WalletTransaction
import org.ippan.wallet.data.TransactionType
import org.ippan.wallet.data.TransactionStatus
import org.ippan.wallet.ui.components.OverviewScreen
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.time.Instant

@RunWith(AndroidJUnit4::class)
class WalletIntegrationTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    @Test
    fun overviewScreen_displaysWalletBalance() {
        // Given
        val mockSnapshot = WalletSnapshot(
            accountAddress = "0x1234567890abcdef",
            totalBalance = 100.0,
            fiatCurrency = "USD",
            tokens = listOf(
                TokenBalance(
                    symbol = "IPP",
                    name = "IPPAN Token",
                    balance = 100.0,
                    fiatValue = 10.0
                )
            ),
            transactions = emptyList(),
            lastSync = Instant.now(),
            activeNode = "https://api.ippan.net"
        )

        // When
        composeTestRule.setContent {
            OverviewScreen(
                state = WalletUiState.Success(
                    address = mockSnapshot.accountAddress,
                    totalBalance = "100.00 IPP",
                    fiatCurrency = mockSnapshot.fiatCurrency,
                    lastSync = mockSnapshot.lastSync,
                    activeEndpoint = mockSnapshot.activeNode,
                    tokens = mockSnapshot.tokens,
                    transactions = mockSnapshot.transactions,
                    lastTransferResult = null
                ),
                onSendClick = {}
            )
        }

        // Then
        composeTestRule.onNodeWithText("100.00 IPP").assertExists()
        composeTestRule.onNodeWithText("0x1234567890abcdef").assertExists()
    }

    @Test
    fun overviewScreen_displaysTransactionHistory() {
        // Given
        val mockTransactions = listOf(
            WalletTransaction(
                id = "tx1",
                type = TransactionType.RECEIVE,
                amount = 50.0,
                symbol = "IPP",
                counterparty = "0xabcdef1234567890",
                timestamp = Instant.now().minusSeconds(3600),
                status = TransactionStatus.CONFIRMED
            ),
            WalletTransaction(
                id = "tx2",
                type = TransactionType.SEND,
                amount = 25.0,
                symbol = "IPP",
                counterparty = "0x9876543210fedcba",
                timestamp = Instant.now().minusSeconds(1800),
                status = TransactionStatus.CONFIRMED
            )
        )

        val mockSnapshot = WalletSnapshot(
            accountAddress = "0x1234567890abcdef",
            totalBalance = 100.0,
            fiatCurrency = "USD",
            tokens = emptyList(),
            transactions = mockTransactions,
            lastSync = Instant.now(),
            activeNode = "https://api.ippan.net"
        )

        // When
        composeTestRule.setContent {
            OverviewScreen(
                state = WalletUiState.Success(
                    address = mockSnapshot.accountAddress,
                    totalBalance = "100.00 IPP",
                    fiatCurrency = mockSnapshot.fiatCurrency,
                    lastSync = mockSnapshot.lastSync,
                    activeEndpoint = mockSnapshot.activeNode,
                    tokens = mockSnapshot.tokens,
                    transactions = mockSnapshot.transactions,
                    lastTransferResult = null
                ),
                onSendClick = {}
            )
        }

        // Then
        composeTestRule.onNodeWithText("+50.00 IPP").assertExists()
        composeTestRule.onNodeWithText("-25.00 IPP").assertExists()
    }
}
