package com.example.myapplication

import com.example.myapplication.domain.EisenhowerCategory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class EisenhowerCategoryTest {
    @Test
    fun `important and urgent is do now`() {
        val category = EisenhowerCategory.from(isImportant = true, isUrgent = true)

        assertEquals(EisenhowerCategory.DO_NOW, category)
        assertTrue(category.isImportant)
        assertTrue(category.isUrgent)
    }

    @Test
    fun `important and not urgent is schedule`() {
        val category = EisenhowerCategory.from(isImportant = true, isUrgent = false)

        assertEquals(EisenhowerCategory.SCHEDULE, category)
        assertTrue(category.isImportant)
        assertFalse(category.isUrgent)
    }

    @Test
    fun `not important and urgent is delegate waiting`() {
        val category = EisenhowerCategory.from(isImportant = false, isUrgent = true)

        assertEquals(EisenhowerCategory.DELEGATE_WAITING, category)
        assertFalse(category.isImportant)
        assertTrue(category.isUrgent)
    }

    @Test
    fun `not important and not urgent is eliminate later`() {
        val category = EisenhowerCategory.from(isImportant = false, isUrgent = false)

        assertEquals(EisenhowerCategory.ELIMINATE_LATER, category)
        assertFalse(category.isImportant)
        assertFalse(category.isUrgent)
    }
}
