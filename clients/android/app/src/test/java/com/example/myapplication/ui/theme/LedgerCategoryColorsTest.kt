package com.example.myapplication.ui.theme

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow

class LedgerCategoryColorsTest {
    @Test
    fun `light and dark category tokens have accessible paired contrast`() {
        assertAccessiblePairs(LightLedgerCategoryColors)
        assertAccessiblePairs(DarkLedgerCategoryColors)
    }

    @Test
    fun `selected composer radio indicator contrasts with its category container`() {
        assertSelectedRadioContrast(LightLedgerCategoryColors)
        assertSelectedRadioContrast(DarkLedgerCategoryColors)
    }

    @Test
    fun `keyboard focus label text contrasts with its category container`() {
        assertFocusLabelContrast(LightLedgerCategoryColors)
        assertFocusLabelContrast(DarkLedgerCategoryColors)
    }

    private fun assertAccessiblePairs(colors: LedgerCategoryColors) {
        assertEquals(4, colors.all.size)
        colors.all.forEach { category ->
            assertTrue(contrastRatio(category.accent, category.onAccent) >= 4.5f)
            assertTrue(contrastRatio(category.container, category.onContainer) >= 4.5f)
            assertTrue(contrastRatio(category.outline, category.container) >= 3f)
            assertTrue(contrastRatio(category.focus, category.container) >= 3f)
        }
    }

    private fun assertSelectedRadioContrast(colors: LedgerCategoryColors) {
        colors.all.forEach { category ->
            assertTrue(contrastRatio(category.accent, category.container) >= 3f)
        }
    }

    private fun assertFocusLabelContrast(colors: LedgerCategoryColors) {
        colors.all.forEach { category ->
            assertTrue(contrastRatio(category.onContainer, category.container) >= 4.5f)
        }
    }

    private fun contrastRatio(first: Color, second: Color): Float {
        val lighter = max(first.luminance(), second.luminance())
        val darker = min(first.luminance(), second.luminance())
        return (lighter + 0.05f) / (darker + 0.05f)
    }

    private fun Color.luminance(): Float =
        (0.2126f * red.toLinear()) + (0.7152f * green.toLinear()) + (0.0722f * blue.toLinear())

    private fun Float.toLinear(): Float =
        if (this <= 0.04045f) this / 12.92f else ((this + 0.055f) / 1.055f).toDouble().pow(2.4).toFloat()
}
