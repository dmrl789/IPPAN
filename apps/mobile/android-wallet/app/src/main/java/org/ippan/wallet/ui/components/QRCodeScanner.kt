package org.ippan.wallet.ui.components

import androidx.camera.core.*
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.FlashOff
import androidx.compose.material.icons.rounded.FlashOn
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * QR Code scanner for wallet addresses
 */
@Composable
fun QRCodeScanner(
    onCodeScanned: (String) -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var isFlashOn by remember { mutableStateOf(false) }
    var isScanning by remember { mutableStateOf(true) }
    
    val cameraExecutor = remember { Executors.newSingleThreadExecutor() }
    
    DisposableEffect(Unit) {
        onDispose {
            cameraExecutor.shutdown()
        }
    }
    
    Box(modifier = modifier.fillMaxSize()) {
        // Camera preview
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx)
                val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
                cameraProviderFuture.addListener({
                    val cameraProvider = cameraProviderFuture.get()
                    
                    val preview = Preview.Builder().build()
                    val imageAnalyzer = ImageAnalysis.Builder()
                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                        .build()
                        .also {
                            it.setAnalyzer(cameraExecutor, QRCodeAnalyzer { code ->
                                if (isScanning) {
                                    isScanning = false
                                    onCodeScanned(code)
                                }
                            })
                        }
                    
                    val cameraSelector = CameraSelector.DEFAULT_BACK_CAMERA
                    
                    try {
                        cameraProvider.unbindAll()
                        cameraProvider.bindToLifecycle(
                            lifecycleOwner,
                            cameraSelector,
                            preview,
                            imageAnalyzer
                        )
                        preview.setSurfaceProvider(previewView.surfaceProvider)
                    } catch (e: Exception) {
                        e.printStackTrace()
                    }
                }, ContextCompat.getMainExecutor(ctx))
                
                previewView
            },
            modifier = Modifier.fillMaxSize()
        )
        
        // Overlay with scanning frame
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color.Black.copy(alpha = 0.3f))
        ) {
            // Scanning frame
            Box(
                modifier = Modifier
                    .size(250.dp)
                    .align(Alignment.Center)
                    .clip(RoundedCornerShape(16.dp))
                    .background(Color.Transparent)
            ) {
                // Corner indicators
                repeat(4) { index ->
                    val cornerSize = 30.dp
                    val cornerColor = MaterialTheme.colorScheme.primary
                    
                    when (index) {
                        0 -> { // Top left
                            Box(
                                modifier = Modifier
                                    .size(cornerSize)
                                    .align(Alignment.TopStart)
                                    .background(Color.Transparent)
                            ) {
                                // Top border
                                Box(
                                    modifier = Modifier
                                        .width(cornerSize)
                                        .height(4.dp)
                                        .background(cornerColor)
                                )
                                // Left border
                                Box(
                                    modifier = Modifier
                                        .width(4.dp)
                                        .height(cornerSize)
                                        .background(cornerColor)
                                )
                            }
                        }
                        1 -> { // Top right
                            Box(
                                modifier = Modifier
                                    .size(cornerSize)
                                    .align(Alignment.TopEnd)
                                    .background(Color.Transparent)
                            ) {
                                // Top border
                                Box(
                                    modifier = Modifier
                                        .width(cornerSize)
                                        .height(4.dp)
                                        .align(Alignment.TopEnd)
                                        .background(cornerColor)
                                )
                                // Right border
                                Box(
                                    modifier = Modifier
                                        .width(4.dp)
                                        .height(cornerSize)
                                        .align(Alignment.BottomEnd)
                                        .background(cornerColor)
                                )
                            }
                        }
                        2 -> { // Bottom left
                            Box(
                                modifier = Modifier
                                    .size(cornerSize)
                                    .align(Alignment.BottomStart)
                                    .background(Color.Transparent)
                            ) {
                                // Bottom border
                                Box(
                                    modifier = Modifier
                                        .width(cornerSize)
                                        .height(4.dp)
                                        .align(Alignment.BottomStart)
                                        .background(cornerColor)
                                )
                                // Left border
                                Box(
                                    modifier = Modifier
                                        .width(4.dp)
                                        .height(cornerSize)
                                        .align(Alignment.TopStart)
                                        .background(cornerColor)
                                )
                            }
                        }
                        3 -> { // Bottom right
                            Box(
                                modifier = Modifier
                                    .size(cornerSize)
                                    .align(Alignment.BottomEnd)
                                    .background(Color.Transparent)
                            ) {
                                // Bottom border
                                Box(
                                    modifier = Modifier
                                        .width(cornerSize)
                                        .height(4.dp)
                                        .align(Alignment.BottomEnd)
                                        .background(cornerColor)
                                )
                                // Right border
                                Box(
                                    modifier = Modifier
                                        .width(4.dp)
                                        .height(cornerSize)
                                        .align(Alignment.TopEnd)
                                        .background(cornerColor)
                                )
                            }
                        }
                    }
                }
            }
        }
        
        // Instructions text
        Text(
            text = "Position the QR code within the frame",
            style = MaterialTheme.typography.bodyLarge,
            color = Color.White,
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = 100.dp)
        )
        
        // Top controls
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
                .align(Alignment.TopCenter),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            IconButton(
                onClick = onDismiss,
                modifier = Modifier.background(
                    Color.Black.copy(alpha = 0.5f),
                    RoundedCornerShape(8.dp)
                )
            ) {
                Icon(
                    imageVector = Icons.Rounded.Close,
                    contentDescription = "Close",
                    tint = Color.White
                )
            }
            
            IconButton(
                onClick = { isFlashOn = !isFlashOn },
                modifier = Modifier.background(
                    Color.Black.copy(alpha = 0.5f),
                    RoundedCornerShape(8.dp)
                )
            ) {
                Icon(
                    imageVector = if (isFlashOn) Icons.Rounded.FlashOn else Icons.Rounded.FlashOff,
                    contentDescription = if (isFlashOn) "Turn off flash" else "Turn on flash",
                    tint = Color.White
                )
            }
        }
    }
}

/**
 * QR Code analyzer for camera
 */
class QRCodeAnalyzer(
    private val onCodeScanned: (String) -> Unit
) : ImageAnalysis.Analyzer {
    
    private val scanner = BarcodeScanning.getClient()
    
    override fun analyze(imageProxy: ImageProxy) {
        val mediaImage = imageProxy.image
        if (mediaImage != null) {
            val image = InputImage.fromMediaImage(mediaImage, imageProxy.imageInfo.rotationDegrees)
            
            scanner.process(image)
                .addOnSuccessListener { barcodes ->
                    for (barcode in barcodes) {
                        barcode.rawValue?.let { value ->
                            if (barcode.valueType == Barcode.TYPE_TEXT || 
                                barcode.valueType == Barcode.TYPE_URL) {
                                onCodeScanned(value)
                            }
                        }
                    }
                }
                .addOnFailureListener { exception ->
                    exception.printStackTrace()
                }
                .addOnCompleteListener {
                    imageProxy.close()
                }
        } else {
            imageProxy.close()
        }
    }
}
