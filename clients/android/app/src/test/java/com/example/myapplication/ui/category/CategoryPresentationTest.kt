package com.example.myapplication.ui.category

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ForwardToInbox
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.LowPriority
import androidx.compose.material.icons.filled.PriorityHigh
import com.example.myapplication.domain.EisenhowerCategory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Test

class CategoryPresentationTest {
    @Test
    fun `every Eisenhower category has canonical and selector labels with its shortcut`() {
        val presentations = EisenhowerCategory.entries.associateWith { it.presentation() }

        assertEquals("Do Now", presentations.getValue(EisenhowerCategory.DO_NOW).label)
        assertEquals("Do Now", presentations.getValue(EisenhowerCategory.DO_NOW).selectorLabel)
        assertEquals("Q", presentations.getValue(EisenhowerCategory.DO_NOW).shortcutLabel)
        assertSame(Icons.Filled.PriorityHigh, presentations.getValue(EisenhowerCategory.DO_NOW).icon)
        assertEquals("Schedule", presentations.getValue(EisenhowerCategory.SCHEDULE).label)
        assertEquals("Schedule", presentations.getValue(EisenhowerCategory.SCHEDULE).selectorLabel)
        assertEquals("W", presentations.getValue(EisenhowerCategory.SCHEDULE).shortcutLabel)
        assertSame(Icons.Filled.Event, presentations.getValue(EisenhowerCategory.SCHEDULE).icon)
        assertEquals("Delegate / Waiting", presentations.getValue(EisenhowerCategory.DELEGATE_WAITING).label)
        assertEquals("Delegate", presentations.getValue(EisenhowerCategory.DELEGATE_WAITING).selectorLabel)
        assertEquals("E", presentations.getValue(EisenhowerCategory.DELEGATE_WAITING).shortcutLabel)
        assertSame(
            Icons.AutoMirrored.Filled.ForwardToInbox,
            presentations.getValue(EisenhowerCategory.DELEGATE_WAITING).icon,
        )
        assertEquals("Eliminate / Later", presentations.getValue(EisenhowerCategory.ELIMINATE_LATER).label)
        assertEquals("Eliminate", presentations.getValue(EisenhowerCategory.ELIMINATE_LATER).selectorLabel)
        assertEquals("R", presentations.getValue(EisenhowerCategory.ELIMINATE_LATER).shortcutLabel)
        assertSame(Icons.Filled.LowPriority, presentations.getValue(EisenhowerCategory.ELIMINATE_LATER).icon)
    }
}
