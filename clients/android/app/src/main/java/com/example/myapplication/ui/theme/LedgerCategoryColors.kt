package com.example.myapplication.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import com.example.myapplication.domain.EisenhowerCategory

@Immutable
data class LedgerCategoryColor(
    val accent: Color,
    val onAccent: Color,
    val container: Color,
    val onContainer: Color,
    val outline: Color,
    val focus: Color,
)

@Immutable
data class LedgerCategoryColors(
    val doNow: LedgerCategoryColor,
    val schedule: LedgerCategoryColor,
    val delegateWaiting: LedgerCategoryColor,
    val eliminateLater: LedgerCategoryColor,
) {
    operator fun get(category: EisenhowerCategory): LedgerCategoryColor = when (category) {
        EisenhowerCategory.DO_NOW -> doNow
        EisenhowerCategory.SCHEDULE -> schedule
        EisenhowerCategory.DELEGATE_WAITING -> delegateWaiting
        EisenhowerCategory.ELIMINATE_LATER -> eliminateLater
    }

    val all: List<LedgerCategoryColor>
        get() = listOf(doNow, schedule, delegateWaiting, eliminateLater)
}

internal val LightLedgerCategoryColors = LedgerCategoryColors(
    doNow = LedgerCategoryColor(
        accent = Color(0xFFB3261E),
        onAccent = Color(0xFFFFFFFF),
        container = Color(0xFFF9DEDC),
        onContainer = Color(0xFF410E0B),
        outline = Color(0xFFB3261E),
        focus = Color(0xFFB3261E),
    ),
    schedule = LedgerCategoryColor(
        accent = Color(0xFF805600),
        onAccent = Color(0xFFFFFFFF),
        container = Color(0xFFFFDEA6),
        onContainer = Color(0xFF2D1B00),
        outline = Color(0xFF9A6700),
        focus = Color(0xFF805600),
    ),
    delegateWaiting = LedgerCategoryColor(
        accent = Color(0xFF1A5EAA),
        onAccent = Color(0xFFFFFFFF),
        container = Color(0xFFD7E3FF),
        onContainer = Color(0xFF001A41),
        outline = Color(0xFF1A5EAA),
        focus = Color(0xFF1A5EAA),
    ),
    eliminateLater = LedgerCategoryColor(
        accent = Color(0xFF5E5E66),
        onAccent = Color(0xFFFFFFFF),
        container = Color(0xFFE6E1E9),
        onContainer = Color(0xFF1C1B20),
        outline = Color(0xFF777680),
        focus = Color(0xFF5E5E66),
    ),
)

internal val DarkLedgerCategoryColors = LedgerCategoryColors(
    doNow = LedgerCategoryColor(
        accent = Color(0xFFFFB4AB),
        onAccent = Color(0xFF690005),
        container = Color(0xFF93000A),
        onContainer = Color(0xFFFFDAD6),
        outline = Color(0xFFFFB4AB),
        focus = Color(0xFFFFB4AB),
    ),
    schedule = LedgerCategoryColor(
        accent = Color(0xFFFFD18A),
        onAccent = Color(0xFF432B00),
        container = Color(0xFF6A4500),
        onContainer = Color(0xFFFFE0B2),
        outline = Color(0xFFFFD18A),
        focus = Color(0xFFFFD18A),
    ),
    delegateWaiting = LedgerCategoryColor(
        accent = Color(0xFFA7C7FF),
        onAccent = Color(0xFF00315F),
        container = Color(0xFF004A77),
        onContainer = Color(0xFFD1E4FF),
        outline = Color(0xFFA7C7FF),
        focus = Color(0xFFA7C7FF),
    ),
    eliminateLater = LedgerCategoryColor(
        accent = Color(0xFFCBC6D0),
        onAccent = Color(0xFF302E35),
        container = Color(0xFF47464E),
        onContainer = Color(0xFFE5E1E9),
        outline = Color(0xFFCBC6D0),
        focus = Color(0xFFCBC6D0),
    ),
)

val LocalLedgerCategoryColors = staticCompositionLocalOf { LightLedgerCategoryColors }

@Composable
fun EisenhowerCategory.ledgerCategoryColors(): LedgerCategoryColor = LocalLedgerCategoryColors.current[this]
