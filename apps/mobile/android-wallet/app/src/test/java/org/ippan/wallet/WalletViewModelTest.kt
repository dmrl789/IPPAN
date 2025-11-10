package org.ippan.wallet

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.ippan.wallet.data.FakeWalletRepository
import org.ippan.wallet.data.TransferRequest
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.JUnit4
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(JUnit4::class)
class WalletViewModelTest {
    
    private lateinit var viewModel: WalletViewModel
    private lateinit var repository: FakeWalletRepository
    
    @Before
    fun setup() {
        repository = FakeWalletRepository()
        viewModel = WalletViewModel(repository)
    }
    
    @Test
    fun `initial state should be loading`() = runTest {
        val initialState = viewModel.uiState.value
        assertTrue(initialState is WalletUiState.Loading)
    }
    
    @Test
    fun `refresh should update wallet state`() = runTest {
        viewModel.refresh()
        
        val state = viewModel.uiState.value
        assertTrue(state is WalletUiState.Success)
        
        val successState = state as WalletUiState.Success
        assertTrue(successState.address.isNotEmpty())
        assertTrue(successState.totalBalance.isNotEmpty())
        assertTrue(successState.tokens.isNotEmpty())
    }
    
    @Test
    fun `submit transfer should update form state`() = runTest {
        // Set up the form state first
        viewModel.updateToAddress("ippan_test123")
        viewModel.updateAmount("100.0")
        viewModel.updateSymbol("IPP")
        viewModel.updateNote("Test transfer")
        
        // Submit without activity (skips biometric auth)
        viewModel.submitTransfer(activity = null)
        
        val formState = viewModel.sendFormState.value
        assertTrue(formState.isSubmitting)
    }
    
    @Test
    fun `update amount should change form state`() {
        viewModel.updateAmount("50.0")
        
        val formState = viewModel.sendFormState.value
        assertEquals("50.0", formState.amount)
    }
    
    @Test
    fun `update address should change form state`() {
        val testAddress = "ippan_test456"
        viewModel.updateToAddress(testAddress)
        
        val formState = viewModel.sendFormState.value
        assertEquals(testAddress, formState.toAddress)
    }
    
    @Test
    fun `update note should change form state`() {
        val testNote = "Test note"
        viewModel.updateNote(testNote)
        
        val formState = viewModel.sendFormState.value
        assertEquals(testNote, formState.note)
    }
    
    @Test
    fun `update symbol should change form state`() {
        val testSymbol = "ETH"
        viewModel.updateSymbol(testSymbol)
        
        val formState = viewModel.sendFormState.value
        assertEquals(testSymbol, formState.symbol)
    }
    
    @Test
    fun `dismiss success banner should clear last transfer result`() = runTest {
        // First set up and submit a transfer to set lastTransferResult
        viewModel.updateToAddress("ippan_test789")
        viewModel.updateAmount("25.0")
        viewModel.updateSymbol("IPP")
        
        viewModel.submitTransfer(activity = null)
        
        // Wait for the transfer to complete
        kotlinx.coroutines.delay(100)
        
        // Dismiss the success banner
        viewModel.dismissSuccessBanner()
        
        val state = viewModel.uiState.value
        if (state is WalletUiState.Success) {
            assertEquals(null, state.lastTransferResult)
        }
    }
}
