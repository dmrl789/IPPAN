package org.ippan.wallet.ui.components

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowCircleDown
import androidx.compose.material.icons.rounded.ArrowCircleUp
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.ippan.wallet.WalletUiState
import org.ippan.wallet.data.TokenBalance
import org.ippan.wallet.data.TransactionStatus
import org.ippan.wallet.data.TransactionType
import org.ippan.wallet.data.WalletTransaction
import org.ippan.wallet.ui.theme.Background
import org.ippan.wallet.ui.theme.Pending
import org.ippan.wallet.ui.theme.Success
import org.ippan.wallet.WalletViewModel.Companion.formatTimestamp

@Composable
fun OverviewScreen(state: WalletUiState, onSendClick: () -> Unit) {
    when (state) {
        is WalletUiState.Success -> OverviewSuccessContent(state, onSendClick)
        WalletUiState.Loading -> OverviewLoadingContent()
        is WalletUiState.Error -> OverviewErrorContent(state.message)
    }
}

@Composable
private fun OverviewSuccessContent(state: WalletUiState.Success, onSendClick: () -> Unit) {
    val clipboard = LocalClipboardManager.current
    val context = LocalContext.current

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(horizontal = 16.dp, vertical = 24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            WalletSummaryCard(
                title = state.totalBalance,
                subtitle = "Total balance",
                address = state.address,
                lastSynced = formatTimestamp(state.lastSync),
                onCopyAddress = {
                    clipboard.setText(AnnotatedString(state.address))
                    Toast.makeText(context, "Address copied", Toast.LENGTH_SHORT).show()
                }
            )
        }

        item {
            QuickActionsRow(
                onSendClick = onSendClick,
                onReceiveClick = {
                    clipboard.setText(AnnotatedString(state.address))
                    Toast.makeText(context, "Address copied", Toast.LENGTH_SHORT).show()
                }
            )
        }

        item {
            Text(
                text = "Assets",
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(bottom = 4.dp)
            )
        }

        items(state.tokens) { token ->
            TokenCard(token)
        }

        item {
            if (state.transactions.isNotEmpty()) {
                RecentTransactionsSection(state.transactions.take(3))
            }
        }
    }
}

@Composable
private fun WalletSummaryCard(
    title: String,
    subtitle: String,
    address: String,
    lastSynced: String,
    onCopyAddress: () -> Unit
) {
    ElevatedCard(
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.primary)
    ) {
        Column(modifier = Modifier.padding(20.dp)) {
            Text(text = subtitle, color = MaterialTheme.colorScheme.onPrimary)
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = title,
                style = MaterialTheme.typography.displayLarge,
                color = MaterialTheme.colorScheme.onPrimary
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "Primary account",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.8f)
            )
            TextButton(onClick = onCopyAddress) {
                Text(text = address, color = MaterialTheme.colorScheme.onPrimary)
            }
            Text(
                text = "Last synced $lastSynced",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.7f)
            )
        }
    }
}

@Composable
private fun QuickActionsRow(onSendClick: () -> Unit, onReceiveClick: () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        FilledTonalButton(onClick = onSendClick) {
            Icon(imageVector = Icons.Rounded.ArrowCircleUp, contentDescription = null)
            Spacer(modifier = Modifier.width(8.dp))
            Text(text = "Send")
        }
        FilledTonalButton(onClick = onReceiveClick) {
            Icon(imageVector = Icons.Rounded.ArrowCircleDown, contentDescription = null)
            Spacer(modifier = Modifier.width(8.dp))
            Text(text = "Receive")
        }
    }
}

@Composable
private fun TokenCard(tokenBalance: TokenBalance) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 20.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(text = tokenBalance.name, style = MaterialTheme.typography.titleMedium)
                Text(
                    text = tokenBalance.symbol.uppercase(),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.outline
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = "${"%,.2f".format(tokenBalance.balance)} ${tokenBalance.symbol.uppercase()}",
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold
                )
                Text(
                    text = "$${"%,.2f".format(tokenBalance.fiatValue)}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.outline
                )
            }
        }
    }
}

@Composable
private fun RecentTransactionsSection(transactions: List<WalletTransaction>) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            text = "Recent activity",
            style = MaterialTheme.typography.headlineMedium
        )
        transactions.forEach { transaction ->
            TransactionRow(transaction)
        }
    }
}

@Composable
fun TransactionRow(transaction: WalletTransaction) {
    val amountColor = when (transaction.type) {
        TransactionType.RECEIVE -> Success
        TransactionType.SEND -> Color(0xFFD14343)
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text(
                    text = if (transaction.type == TransactionType.SEND) "Sent" else "Received",
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = "${transaction.counterparty}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.outline,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    text = (if (transaction.type == TransactionType.SEND) "-" else "+") +
                        "${"%,.2f".format(transaction.amount)} ${transaction.symbol}",
                    color = amountColor,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold
                )
                val statusColor = if (transaction.status == TransactionStatus.CONFIRMED) Success else Pending
                val statusLabel = if (transaction.status == TransactionStatus.CONFIRMED) "Confirmed" else "Pending"
                Text(
                    text = "$statusLabel • ${formatTimestamp(transaction.timestamp)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = statusColor
                )
            }
        }
    }
}

@Composable
private fun OverviewLoadingContent() {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        contentAlignment = Alignment.Center
    ) {
        CircularProgressIndicator()
    }
}

@Composable
private fun OverviewErrorContent(message: String) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.error
        )
    }
}
