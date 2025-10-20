package org.ippan.wallet

import android.content.Context
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import org.ippan.wallet.data.TokenBalance
import org.ippan.wallet.data.TransferRequest
import org.ippan.wallet.data.WalletRepository
import org.ippan.wallet.data.WalletTransaction
import org.ippan.wallet.security.BiometricAuthManager
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

sealed interface WalletUiState {
    data object Loading : WalletUiState
    data class Success(
        val address: String,
        val totalBalance: String,
        val fiatCurrency: String,
        val lastSync: Instant,
        val activeEndpoint: String,
        val tokens: List<TokenBalance>,
        val transactions: List<WalletTransaction>,
        val lastTransferResult: TransferResult?
    ) : WalletUiState

    data class Error(val message: String) : WalletUiState
}

data class TransferResult(
    val amount: Double,
    val symbol: String,
    val toAddress: String,
    val submittedAt: Instant
)

data class SendFormState(
    val toAddress: String = "",
    val amount: String = "",
    val symbol: String = "IPP",
    val note: String = "",
    val isSubmitting: Boolean = false,
    val isAuthenticating: Boolean = false,
    val error: String? = null
)

class WalletViewModel(
    private val repository: WalletRepository,
    private val biometricAuthManager: BiometricAuthManager? = null
) : ViewModel() {

    private val _uiState: MutableStateFlow<WalletUiState> = MutableStateFlow(WalletUiState.Loading)
    val uiState: StateFlow<WalletUiState> = _uiState.asStateFlow()

    private val _sendFormState = MutableStateFlow(SendFormState())
    val sendFormState: StateFlow<SendFormState> = _sendFormState.asStateFlow()

    init {
        observeWallet()
        refresh()
    }

    private fun observeWallet() {
        viewModelScope.launch {
            repository.snapshot().collectLatest { snapshot ->
                val previous = _uiState.value
                val lastTransfer = if (previous is WalletUiState.Success) previous.lastTransferResult else null
                _uiState.value = WalletUiState.Success(
                    address = snapshot.accountAddress,
                    totalBalance = formatCurrency(snapshot.totalBalance, snapshot.fiatCurrency),
                    fiatCurrency = snapshot.fiatCurrency,
                    lastSync = snapshot.lastSync,
                    activeEndpoint = snapshot.activeNode,
                    tokens = snapshot.tokens,
                    transactions = snapshot.transactions,
                    lastTransferResult = lastTransfer
                )
            }
        }
    }

    fun refresh() {
        viewModelScope.launch {
            try {
                repository.refresh()
            } catch (ex: Exception) {
                _uiState.value = WalletUiState.Error(ex.message ?: "Unable to refresh wallet")
            }
        }
    }

    fun updateToAddress(address: String) {
        _sendFormState.value = _sendFormState.value.copy(toAddress = address, error = null)
    }

    fun updateAmount(amount: String) {
        _sendFormState.value = _sendFormState.value.copy(amount = amount, error = null)
    }

    fun updateSymbol(symbol: String) {
        _sendFormState.value = _sendFormState.value.copy(symbol = symbol.uppercase(), error = null)
    }

    fun updateNote(note: String) {
        _sendFormState.value = _sendFormState.value.copy(note = note)
    }

    fun submitTransfer(activity: FragmentActivity? = null) {
        val state = _sendFormState.value
        val amount = state.amount.toDoubleOrNull()
        if (state.toAddress.isBlank()) {
            _sendFormState.value = state.copy(error = "Destination address required")
            return
        }
        if (amount == null || amount <= 0.0) {
            _sendFormState.value = state.copy(error = "Enter a valid amount")
            return
        }

        _sendFormState.value = state.copy(isSubmitting = true, error = null)
        viewModelScope.launch {
            try {
                // Check if biometric authentication is required and available
                if (biometricAuthManager?.isBiometricAvailable() == true && activity != null) {
                    _sendFormState.value = state.copy(isAuthenticating = true, isSubmitting = false)
                    
                    val authResult = biometricAuthManager.authenticateForTransaction(activity)
                    if (authResult.isFailure) {
                        _sendFormState.value = state.copy(
                            isAuthenticating = false,
                            isSubmitting = false,
                            error = "Authentication failed. Please try again."
                        )
                        return@launch
                    }
                    
                    _sendFormState.value = state.copy(isAuthenticating = false, isSubmitting = true)
                }

                repository.submitTransfer(
                    TransferRequest(
                        toAddress = state.toAddress,
                        amount = amount,
                        symbol = state.symbol,
                        note = state.note.ifBlank { null }
                    )
                )
                _sendFormState.value = SendFormState(symbol = state.symbol)
                val successState = _uiState.value
                if (successState is WalletUiState.Success) {
                    _uiState.value = successState.copy(
                        lastTransferResult = TransferResult(
                            amount = amount,
                            symbol = state.symbol,
                            toAddress = state.toAddress,
                            submittedAt = Instant.now()
                        )
                    )
                }
            } catch (ex: Exception) {
                _sendFormState.value = state.copy(
                    isSubmitting = false,
                    isAuthenticating = false,
                    error = ex.message ?: "Failed to submit transaction"
                )
            }
        }
    }

    fun dismissSuccessBanner() {
        val current = _uiState.value
        if (current is WalletUiState.Success) {
            _uiState.value = current.copy(lastTransferResult = null)
        }
    }

    companion object {
        private val displayFormatter = DateTimeFormatter.ofPattern("MMM d, HH:mm")
            .withZone(ZoneId.systemDefault())

        fun formatTimestamp(timestamp: Instant): String = displayFormatter.format(timestamp)

        private fun formatCurrency(amount: Double, currency: String): String {
            return "${"%,.2f".format(amount)} $currency"
        }
    }
}

class WalletViewModelFactory(
    private val repositoryProvider: () -> WalletRepository,
    private val biometricAuthManager: BiometricAuthManager? = null
) : androidx.lifecycle.ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(WalletViewModel::class.java)) {
            return WalletViewModel(repositoryProvider(), biometricAuthManager) as T
        }
        throw IllegalArgumentException("Unknown ViewModel class: ${modelClass.name}")
    }
}
