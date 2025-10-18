package org.ippan.wallet.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun SettingsScreen(activeEndpoint: String, onRefreshClick: () -> Unit) {
    val biometricLogin = remember { mutableStateOf(true) }
    val pushNotifications = remember { mutableStateOf(true) }

    Column(
        modifier = Modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "Preferences",
            style = MaterialTheme.typography.headlineMedium
        )
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = "Network",
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = "Connected to",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.outline
                )
                Text(
                    text = activeEndpoint,
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }
        SettingsRow(
            title = "Biometric unlock",
            description = "Require biometrics when sending funds",
            checked = biometricLogin.value,
            onCheckedChange = { biometricLogin.value = it }
        )
        SettingsRow(
            title = "Push notifications",
            description = "Get notified when a transfer is confirmed",
            checked = pushNotifications.value,
            onCheckedChange = { pushNotifications.value = it }
        )
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = "Maintenance",
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = "Resync the wallet if balances look outdated.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.outline
                )
                Button(onClick = onRefreshClick) {
                    Text("Refresh now")
                }
            }
        }
    }
}

@Composable
private fun SettingsRow(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(text = title, style = MaterialTheme.typography.titleMedium)
            Text(text = description, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.outline)
            Switch(checked = checked, onCheckedChange = onCheckedChange)
        }
    }
}
