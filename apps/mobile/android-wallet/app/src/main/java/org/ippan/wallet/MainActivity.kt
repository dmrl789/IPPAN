package org.ippan.wallet

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AccountBalanceWallet
import androidx.compose.material.icons.rounded.ArrowCircleUp
import androidx.compose.material.icons.rounded.Autorenew
import androidx.compose.material.icons.rounded.History
import androidx.compose.material.icons.rounded.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.IconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberSnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavDestination
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.NavHostController
import org.ippan.wallet.ui.components.ActivityScreen
import org.ippan.wallet.ui.components.OverviewScreen
import org.ippan.wallet.ui.components.SendTokenSheet
import org.ippan.wallet.ui.components.SettingsScreen
import org.ippan.wallet.ui.theme.IppanWalletTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { IppanWalletApp() }
    }
}

enum class WalletDestination(val route: String, val label: String) {
    Overview("overview", "Overview"),
    Activity("activity", "Activity"),
    Settings("settings", "Settings")
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun IppanWalletApp(viewModel: WalletViewModel = viewModel()) {
    IppanWalletTheme {
        val navController = rememberNavController()
        val navBackStackEntry by navController.currentBackStackEntryAsState()
        val currentDestination = navBackStackEntry?.destination

        val snackbarHostState: SnackbarHostState = rememberSnackbarHostState()
        val uiState by viewModel.uiState.collectAsStateWithLifecycle()
        val sendForm by viewModel.sendFormState.collectAsStateWithLifecycle()

        var isSendSheetVisible by remember { mutableStateOf(false) }

        LaunchedEffect(uiState) {
            if (uiState is WalletUiState.Success) {
                val lastTransfer = (uiState as WalletUiState.Success).lastTransferResult
                if (lastTransfer != null) {
                    isSendSheetVisible = false
                    snackbarHostState.showSnackbar(
                        message = "Sent ${"%,.2f".format(lastTransfer.amount)} ${lastTransfer.symbol} to ${lastTransfer.toAddress}",
                        withDismissAction = true
                    )
                    viewModel.dismissSuccessBanner()
                }
            } else if (uiState is WalletUiState.Error) {
                snackbarHostState.showSnackbar((uiState as WalletUiState.Error).message)
            }
        }

        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text(text = "IPPAN Wallet") },
                    actions = {
                        IconButton(onClick = viewModel::refresh) {
                            Icon(
                                imageVector = Icons.Rounded.Autorenew,
                                contentDescription = "Refresh"
                            )
                        }
                    }
                )
            },
            snackbarHost = { SnackbarHost(snackbarHostState) },
            floatingActionButton = {
                ExtendedFloatingActionButton(
                    text = { Text("Send") },
                    icon = {
                        Icon(
                            imageVector = Icons.Rounded.ArrowCircleUp,
                            contentDescription = "Send"
                        )
                    },
                    onClick = { isSendSheetVisible = true }
                )
            },
            bottomBar = {
                NavigationBar {
                    WalletDestination.values().forEach { destination ->
                        NavigationBarItem(
                            icon = {
                                Icon(
                                    imageVector = when (destination) {
                                        WalletDestination.Overview -> Icons.Rounded.AccountBalanceWallet
                                        WalletDestination.Activity -> Icons.Rounded.History
                                        WalletDestination.Settings -> Icons.Rounded.Settings
                                    },
                                    contentDescription = destination.label
                                )
                            },
                            label = { Text(destination.label) },
                            selected = currentDestination.isInHierarchy(destination.route),
                            onClick = {
                                navController.navigate(destination.route) {
                                    popUpTo(navController.graph.findStartDestination().id) {
                                        saveState = true
                                    }
                                    launchSingleTop = true
                                    restoreState = true
                                }
                            }
                        )
                    }
                }
            }
        ) { innerPadding ->
            WalletNavHost(
                modifier = Modifier.padding(innerPadding),
                navController = navController,
                uiState = uiState,
                onSendClick = { isSendSheetVisible = true },
                onRefresh = viewModel::refresh
            )
        }

        if (isSendSheetVisible) {
            SendTokenSheet(
                state = sendForm,
                onDismiss = { isSendSheetVisible = false },
                onSubmit = viewModel::submitTransfer,
                onAmountChange = viewModel::updateAmount,
                onAddressChange = viewModel::updateToAddress,
                onNoteChange = viewModel::updateNote,
                onSymbolChange = viewModel::updateSymbol
            )
        }
    }
}

@Composable
private fun WalletNavHost(
    modifier: Modifier,
    navController: NavHostController,
    uiState: WalletUiState,
    onSendClick: () -> Unit,
    onRefresh: () -> Unit
) {
    Box(modifier = modifier.fillMaxSize()) {
        NavHost(
            navController = navController,
            startDestination = WalletDestination.Overview.route
        ) {
            composable(WalletDestination.Overview.route) {
                OverviewRoute(uiState = uiState, onSendClick = onSendClick)
            }
            composable(WalletDestination.Activity.route) {
                ActivityRoute(uiState = uiState)
            }
            composable(WalletDestination.Settings.route) {
                SettingsRoute(onRefreshClick = onRefresh)
            }
        }
    }
}

@Composable
private fun OverviewRoute(uiState: WalletUiState, onSendClick: () -> Unit) {
    when (uiState) {
        is WalletUiState.Success -> OverviewScreen(state = uiState, onSendClick = onSendClick)
        WalletUiState.Loading -> LoadingScreen()
        is WalletUiState.Error -> ErrorScreen(uiState.message)
    }
}

@Composable
private fun ActivityRoute(uiState: WalletUiState) {
    when (uiState) {
        is WalletUiState.Success -> ActivityScreen(transactions = uiState.transactions)
        WalletUiState.Loading -> LoadingScreen()
        is WalletUiState.Error -> ErrorScreen(uiState.message)
    }
}

@Composable
private fun SettingsRoute(onRefreshClick: () -> Unit) {
    SettingsScreen(onRefreshClick = onRefreshClick)
}

@Composable
private fun LoadingScreen() {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        contentAlignment = Alignment.Center
    ) {
        androidx.compose.material3.CircularProgressIndicator()
    }
}

@Composable
private fun ErrorScreen(message: String) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(text = message)
    }
}

private fun NavDestination?.isInHierarchy(route: String): Boolean {
    return this?.hierarchy?.any { it.route == route } == true
}
