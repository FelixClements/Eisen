package com.example.myapplication.ui.home

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task

object HomeTaskSorter {
    private val taskComparator = compareBy<Task> { task -> if (task.isPinned) 0 else 1 }
        .thenBy { task -> if (task.dueDate == null) 1 else 0 }
        .thenBy { task -> task.dueDate ?: Long.MAX_VALUE }
        .thenByDescending { task -> task.createdAt }

    fun groupTasks(tasks: List<Task>): Map<EisenhowerCategory, List<Task>> {
        val groupedTasks = tasks.groupBy { task -> task.eisenhowerCategory }

        return EisenhowerCategory.entries.associateWith { category ->
            groupedTasks[category].orEmpty().sortedWith(taskComparator)
        }
    }
}
