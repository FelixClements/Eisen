package com.example.myapplication.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider

/** A static theme for Compose UI tests that must not depend on device dynamic colors. */
@Composable
fun StaticTestTheme(
    darkTheme: Boolean = false,
    content: @Composable () -> Unit,
) {
    val ledgerCategoryColors = if (darkTheme) {
        DarkLedgerCategoryColors
    } else {
        LightLedgerCategoryColors
    }

    CompositionLocalProvider(LocalLedgerCategoryColors provides ledgerCategoryColors) {
        MaterialTheme(
            colorScheme = if (darkTheme) darkColorScheme() else lightColorScheme(),
            content = content,
        )
    }
}
