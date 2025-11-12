package org.ippan.wallet.ui

import app.cash.paparazzi.Paparazzi
import org.ippan.wallet.data.TokenBalance
import org.ippan.wallet.data.WalletTransaction
import org.ippan.wallet.data.TransactionType
import org.ippan.wallet.data.TransactionStatus
import org.ippan.wallet.ui.components.ActivityScreen
import org.ippan.wallet.ui.components.OverviewScreen
import org.ippan.wallet.ui.components.SendTokenSheet
import org.ippan.wallet.ui.components.SettingsScreen
import org.ippan.wallet.ui.theme.IppanWalletTheme
import org.ippan.wallet.WalletUiState
import org.ippan.wallet.SendFormState
import org.junit.Ignore
import org.junit.Rule
import org.junit.Test
import java.time.Instant

// Paparazzi snapshot tests require special setup and resources
// These should be run separately with proper Android test configuration
@Ignore("Paparazzi tests require special resource configuration")
class WalletScreenshotsTest {

    @get:Rule
    val paparazzi = Paparazzi()

    @Test
    fun overviewScreen() {
        val mockSnapshot = createMockWalletSnapshot()
        
        paparazzi.snapshot {
            IppanWalletTheme {
                OverviewScreen(
                    state = WalletUiState.Success(
                        address = mockSnapshot.accountAddress,
                        totalBalance = "1,250.50 IPP",
                        fiatCurrency = "USD",
                        lastSync = mockSnapshot.lastSync,
                        activeEndpoint = mockSnapshot.activeNode,
                        tokens = mockSnapshot.tokens,
                        transactions = mockSnapshot.transactions,
                        lastTransferResult = null
                    ),
                    onSendClick = {}
                )
            }
        }
    }

    @Test
    fun overviewScreenWithTransaction() {
        val mockSnapshot = createMockWalletSnapshot()
        
        paparazzi.snapshot {
            IppanWalletTheme {
                OverviewScreen(
                    state = WalletUiState.Success(
                        address = mockSnapshot.accountAddress,
                        totalBalance = "1,250.50 IPP",
                        fiatCurrency = "USD",
                        lastSync = mockSnapshot.lastSync,
                        activeEndpoint = mockSnapshot.activeNode,
                        tokens = mockSnapshot.tokens,
                        transactions = mockSnapshot.transactions,
                        lastTransferResult = org.ippan.wallet.TransferResult(
                            amount = 100.0,
                            symbol = "IPP",
                            toAddress = "0x9876543210fedcba",
                            submittedAt = Instant.now()
                        )
                    ),
                    onSendClick = {}
                )
            }
        }
    }

    @Test
    fun activityScreen() {
        val mockTransactions = createMockTransactions()
        
        paparazzi.snapshot {
            IppanWalletTheme {
                ActivityScreen(transactions = mockTransactions)
            }
        }
    }

    @Test
    fun sendTokenSheet() {
        paparazzi.snapshot {
            IppanWalletTheme {
                SendTokenSheet(
                    state = SendFormState(
                        toAddress = "0x1234567890abcdef",
                        amount = "100.0",
                        symbol = "IPP",
                        note = "Payment for services",
                        isSubmitting = false,
                        isAuthenticating = false,
                        error = null
                    ),
                    onDismiss = {},
                    onSubmit = {},
                    onAmountChange = {},
                    onAddressChange = {},
                    onNoteChange = {},
                    onSymbolChange = {}
                )
            }
        }
    }

    @Test
    fun sendTokenSheetWithError() {
        paparazzi.snapshot {
            IppanWalletTheme {
                SendTokenSheet(
                    state = SendFormState(
                        toAddress = "invalid_address",
                        amount = "1000.0",
                        symbol = "IPP",
                        note = "",
                        isSubmitting = false,
                        isAuthenticating = false,
                        error = "Invalid address format"
                    ),
                    onDismiss = {},
                    onSubmit = {},
                    onAmountChange = {},
                    onAddressChange = {},
                    onNoteChange = {},
                    onSymbolChange = {}
                )
            }
        }
    }

    @Test
    fun sendTokenSheetAuthenticating() {
        paparazzi.snapshot {
            IppanWalletTheme {
                SendTokenSheet(
                    state = SendFormState(
                        toAddress = "0x1234567890abcdef",
                        amount = "50.0",
                        symbol = "IPP",
                        note = "",
                        isSubmitting = false,
                        isAuthenticating = true,
                        error = null
                    ),
                    onDismiss = {},
                    onSubmit = {},
                    onAmountChange = {},
                    onAddressChange = {},
                    onNoteChange = {},
                    onSymbolChange = {}
                )
            }
        }
    }

    @Test
    fun settingsScreen() {
        paparazzi.snapshot {
            IppanWalletTheme {
                SettingsScreen(
                    activeEndpoint = "https://api.ippan.net",
                    onRefreshClick = {}
                )
            }
        }
    }

    @Test
    fun loadingState() {
        paparazzi.snapshot {
            IppanWalletTheme {
                OverviewScreen(
                    state = WalletUiState.Loading,
                    onSendClick = {}
                )
            }
        }
    }

    @Test
    fun errorState() {
        paparazzi.snapshot {
            IppanWalletTheme {
                OverviewScreen(
                    state = WalletUiState.Error("Unable to connect to network"),
                    onSendClick = {}
                )
            }
        }
    }

    private fun createMockWalletSnapshot() = org.ippan.wallet.data.WalletSnapshot(
        accountAddress = "0x1234567890abcdef1234567890abcdef12345678",
        totalBalance = 1250.50,
        fiatCurrency = "USD",
        tokens = listOf(
            TokenBalance(
                symbol = "IPP",
                name = "IPPAN Token",
                balance = 1250.50,
                fiatValue = 125.05
            )
        ),
        transactions = createMockTransactions(),
        lastSync = Instant.now().minusSeconds(300),
        activeNode = "https://api.ippan.net"
    )

    private fun createMockTransactions() = listOf(
        WalletTransaction(
            id = "tx_001",
            type = TransactionType.RECEIVE,
            amount = 500.0,
            symbol = "IPP",
            counterparty = "0xabcdef1234567890abcdef1234567890abcdef12",
            timestamp = Instant.now().minusSeconds(3600),
            status = TransactionStatus.CONFIRMED
        ),
        WalletTransaction(
            id = "tx_002",
            type = TransactionType.SEND,
            amount = 100.0,
            symbol = "IPP",
            counterparty = "0x9876543210fedcba9876543210fedcba98765432",
            timestamp = Instant.now().minusSeconds(1800),
            status = TransactionStatus.CONFIRMED
        ),
        WalletTransaction(
            id = "tx_003",
            type = TransactionType.RECEIVE,
            amount = 250.0,
            symbol = "IPP",
            counterparty = "0x5555555555555555555555555555555555555555",
            timestamp = Instant.now().minusSeconds(900),
            status = TransactionStatus.PENDING
        ),
        WalletTransaction(
            id = "tx_004",
            type = TransactionType.SEND,
            amount = 75.0,
            symbol = "IPP",
            counterparty = "0x3333333333333333333333333333333333333333",
            timestamp = Instant.now().minusSeconds(300),
            status = TransactionStatus.CONFIRMED
        )
    )
}
