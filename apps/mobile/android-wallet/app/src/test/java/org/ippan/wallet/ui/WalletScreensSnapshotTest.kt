package org.ippan.wallet.ui

import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import app.cash.paparazzi.RenderingMode
import java.time.Instant
import org.junit.Rule
import org.junit.Test
import org.ippan.wallet.SendFormState
import org.ippan.wallet.TransferResult
import org.ippan.wallet.WalletUiState
import org.ippan.wallet.data.TokenBalance
import org.ippan.wallet.data.TransactionStatus
import org.ippan.wallet.data.TransactionType
import org.ippan.wallet.data.WalletTransaction
import org.ippan.wallet.ui.components.ActivityScreen
import org.ippan.wallet.ui.components.OverviewScreen
import org.ippan.wallet.ui.components.SendTokenSheet
import org.ippan.wallet.ui.components.SettingsScreen
import org.ippan.wallet.ui.theme.IppanWalletTheme

class WalletScreensSnapshotTest {

    @get:Rule
    val paparazzi = Paparazzi(
        deviceConfig = DeviceConfig.PIXEL_6.copy(softButtons = false),
        renderingMode = RenderingMode.SHRINK
    )

    private val sampleTransactions = listOf(
        WalletTransaction(
            id = "tx-01",
            type = TransactionType.RECEIVE,
            amount = 128.0,
            symbol = "IPP",
            counterparty = "validator.alpha",
            timestamp = Instant.parse("2025-10-17T12:00:00Z"),
            status = TransactionStatus.CONFIRMED
        ),
        WalletTransaction(
            id = "tx-02",
            type = TransactionType.SEND,
            amount = 25.5,
            symbol = "IPP",
            counterparty = "merchant.beta",
            timestamp = Instant.parse("2025-10-15T09:12:00Z"),
            status = TransactionStatus.PENDING
        )
    )

    private val sampleState = WalletUiState.Success(
        address = "ippan1qxyza0address",
        totalBalance = "1,234.00 IPP",
        fiatCurrency = "IPP",
        lastSync = Instant.parse("2025-10-17T12:05:00Z"),
        activeEndpoint = "https://api.ippan.net",
        tokens = listOf(
            TokenBalance("IPP", "IPPAN", 1234.0, 123.4),
            TokenBalance("NDR", "Node Runner", 85.0, 212.5)
        ),
        transactions = sampleTransactions,
        lastTransferResult = TransferResult(
            amount = 5.0,
            symbol = "IPP",
            toAddress = "ippan1qrecipient",
            submittedAt = Instant.parse("2025-10-17T11:55:00Z")
        )
    )

    private val sendForm = SendFormState(
        toAddress = "ippan1qrecipient",
        amount = "42.00",
        symbol = "IPP",
        note = "Payment for services",
        isSubmitting = false,
        error = null
    )

    @Test
    fun overviewScreen_snapshot() {
        paparazzi.snapshot("overview") {
            IppanWalletTheme {
                OverviewScreen(state = sampleState, onSendClick = {})
            }
        }
    }

    @Test
    fun activityScreen_snapshot() {
        paparazzi.snapshot("activity") {
            IppanWalletTheme {
                ActivityScreen(transactions = sampleTransactions)
            }
        }
    }

    @Test
    fun settingsScreen_snapshot() {
        paparazzi.snapshot("settings") {
            IppanWalletTheme {
                SettingsScreen(activeEndpoint = sampleState.activeEndpoint, onRefreshClick = {})
            }
        }
    }

    @Test
    fun sendSheet_snapshot() {
        paparazzi.snapshot("send_sheet") {
            IppanWalletTheme {
                SendTokenSheet(
                    state = sendForm,
                    onDismiss = {},
                    onSubmit = {},
                    onAmountChange = {},
                    onAddressChange = {},
                    onSymbolChange = {},
                    onNoteChange = {}
                )
            }
        }
    }
}
