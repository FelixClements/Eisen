package com.example.myapplication.ui.home

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeTaskSorterTest {
    @Test
    fun `groupTasks groups tasks into all categories`() {
        val tasks = EisenhowerCategory.entries.mapIndexed { index, category ->
            task(
                id = index.toLong(),
                title = category.name,
                category = category,
            )
        }

        val groupedTasks = HomeTaskSorter.groupTasks(tasks)
        val emptyGroupedTasks = HomeTaskSorter.groupTasks(emptyList())

        assertEquals(EisenhowerCategory.entries.toSet(), groupedTasks.keys)
        EisenhowerCategory.entries.forEach { category ->
            assertEquals(listOf(category.name), groupedTasks.getValue(category).map { it.title })
        }
        assertEquals(EisenhowerCategory.entries.toSet(), emptyGroupedTasks.keys)
        assertTrue(emptyGroupedTasks.values.all { it.isEmpty() })
    }

    @Test
    fun `groupTasks sorts pinned first then due dates then newest created`() {
        val tasks = listOf(
            task(id = 1L, title = "Due later", dueDate = 300L, createdAt = 50L),
            task(id = 2L, title = "Due early old", dueDate = 100L, createdAt = 20L),
            task(id = 3L, title = "Pinned undated", isPinned = true, dueDate = null, createdAt = 10L),
            task(id = 4L, title = "Undated old", dueDate = null, createdAt = 10L),
            task(id = 5L, title = "Undated new", dueDate = null, createdAt = 20L),
            task(id = 6L, title = "Due early new", dueDate = 100L, createdAt = 30L),
        )

        val sortedIds = HomeTaskSorter.groupTasks(tasks)
            .getValue(EisenhowerCategory.DO_NOW)
            .map { task -> task.id }

        assertEquals(listOf(3L, 6L, 2L, 1L, 5L, 4L), sortedIds)
    }

    private fun task(
        id: Long,
        title: String = "Task $id",
        category: EisenhowerCategory = EisenhowerCategory.DO_NOW,
        isPinned: Boolean = false,
        dueDate: Long? = null,
        createdAt: Long = id,
    ): Task = Task(
        id = id,
        title = title,
        description = null,
        isImportant = category.isImportant,
        isUrgent = category.isUrgent,
        dueDate = dueDate,
        isCompleted = false,
        isArchived = false,
        isPinned = isPinned,
        category = null,
        createdAt = createdAt,
        updatedAt = createdAt,
    )
}
