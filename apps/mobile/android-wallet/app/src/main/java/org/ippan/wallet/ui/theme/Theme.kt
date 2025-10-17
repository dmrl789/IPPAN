package org.ippan.wallet.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

private val DarkColorScheme = darkColorScheme(
    primary = PrimaryBlue,
    onPrimary = PrimaryOnBlue,
    secondary = AccentGold,
    background = OnSurface,
    surface = Surface,
    onSurface = PrimaryOnBlue,
    outline = Outline
)

private val LightColorScheme = lightColorScheme(
    primary = PrimaryBlue,
    onPrimary = PrimaryOnBlue,
    secondary = AccentGold,
    background = Background,
    surface = Surface,
    onSurface = OnSurface,
    outline = Outline
)

@Composable
fun IppanWalletTheme(
    useDarkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (useDarkTheme) DarkColorScheme else LightColorScheme

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
