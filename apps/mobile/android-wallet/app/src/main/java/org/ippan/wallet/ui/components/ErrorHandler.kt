package org.ippan.wallet.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Error
import androidx.compose.material.icons.rounded.Refresh
import androidx.compose.material.icons.rounded.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import org.ippan.wallet.data.WalletError

/**
 * Comprehensive error handling and user feedback system
 */
@Composable
fun ErrorCard(
    error: WalletError,
    onRetry: (() -> Unit)? = null,
    onDismiss: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = when (error.severity) {
                WalletError.Severity.ERROR -> MaterialTheme.colorScheme.errorContainer
                WalletError.Severity.WARNING -> MaterialTheme.colorScheme.tertiaryContainer
                WalletError.Severity.INFO -> MaterialTheme.colorScheme.primaryContainer
            }
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Icon(
                    imageVector = when (error.severity) {
                        WalletError.Severity.ERROR -> Icons.Rounded.Error
                        WalletError.Severity.WARNING -> Icons.Rounded.Warning
                        WalletError.Severity.INFO -> Icons.Rounded.Warning
                    },
                    contentDescription = null,
                    tint = when (error.severity) {
                        WalletError.Severity.ERROR -> MaterialTheme.colorScheme.onErrorContainer
                        WalletError.Severity.WARNING -> MaterialTheme.colorScheme.onTertiaryContainer
                        WalletError.Severity.INFO -> MaterialTheme.colorScheme.onPrimaryContainer
                    }
                )
                Text(
                    text = error.title,
                    style = MaterialTheme.typography.titleMedium,
                    color = when (error.severity) {
                        WalletError.Severity.ERROR -> MaterialTheme.colorScheme.onErrorContainer
                        WalletError.Severity.WARNING -> MaterialTheme.colorScheme.onTertiaryContainer
                        WalletError.Severity.INFO -> MaterialTheme.colorScheme.onPrimaryContainer
                    }
                )
            }
            
            Text(
                text = error.message,
                style = MaterialTheme.typography.bodyMedium,
                color = when (error.severity) {
                    WalletError.Severity.ERROR -> MaterialTheme.colorScheme.onErrorContainer
                    WalletError.Severity.WARNING -> MaterialTheme.colorScheme.onTertiaryContainer
                    WalletError.Severity.INFO -> MaterialTheme.colorScheme.onPrimaryContainer
                }
            )
            
            if (error.suggestion != null) {
                Text(
                    text = "💡 ${error.suggestion}",
                    style = MaterialTheme.typography.bodySmall,
                    color = when (error.severity) {
                        WalletError.Severity.ERROR -> MaterialTheme.colorScheme.onErrorContainer
                        WalletError.Severity.WARNING -> MaterialTheme.colorScheme.onTertiaryContainer
                        WalletError.Severity.INFO -> MaterialTheme.colorScheme.onPrimaryContainer
                    }
                )
            }
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                if (onDismiss != null) {
                    TextButton(
                        onClick = onDismiss,
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Dismiss")
                    }
                }
                
                if (onRetry != null && error.isRetryable) {
                    OutlinedButton(
                        onClick = onRetry,
                        modifier = Modifier.weight(1f)
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.Refresh,
                            contentDescription = null,
                            modifier = Modifier.width(16.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text("Retry")
                    }
                }
            }
        }
    }
}

@Composable
fun ErrorDialog(
    error: WalletError,
    onDismiss: () -> Unit,
    onRetry: (() -> Unit)? = null,
    onConfirm: (() -> Unit)? = null
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Icon(
                    imageVector = when (error.severity) {
                        WalletError.Severity.ERROR -> Icons.Rounded.Error
                        WalletError.Severity.WARNING -> Icons.Rounded.Warning
                        WalletError.Severity.INFO -> Icons.Rounded.Warning
                    },
                    contentDescription = null
                )
                Text(text = error.title)
            }
        },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(text = error.message)
                if (error.suggestion != null) {
                    Text(
                        text = "💡 ${error.suggestion}",
                        style = MaterialTheme.typography.bodySmall
                    )
                }
            }
        },
        confirmButton = {
            if (onConfirm != null) {
                Button(onClick = onConfirm) {
                    Text("OK")
                }
            } else if (onRetry != null && error.isRetryable) {
                Button(onClick = onRetry) {
                    Icon(
                        imageVector = Icons.Rounded.Refresh,
                        contentDescription = null,
                        modifier = Modifier.width(16.dp)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("Retry")
                }
            } else {
                Button(onClick = onDismiss) {
                    Text("OK")
                }
            }
        },
        dismissButton = {
            if (onConfirm != null || (onRetry != null && error.isRetryable)) {
                TextButton(onClick = onDismiss) {
                    Text("Cancel")
                }
            }
        }
    )
}

@Composable
fun EmptyStateCard(
    title: String,
    message: String,
    icon: ImageVector,
    actionText: String? = null,
    onAction: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier
                .padding(24.dp)
                .fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.width(48.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Text(
                text = title,
                style = MaterialTheme.typography.headlineSmall,
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                textAlign = TextAlign.Center,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            if (actionText != null && onAction != null) {
                Button(onClick = onAction) {
                    Text(actionText)
                }
            }
        }
    }
}
