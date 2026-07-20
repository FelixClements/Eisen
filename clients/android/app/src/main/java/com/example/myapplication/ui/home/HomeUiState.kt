package com.example.myapplication.ui.home

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task

data class HomeUiState(
    val activeTasks: List<Task> = emptyList(),
    val groupedTasks: Map<EisenhowerCategory, List<Task>> = HomeTaskSorter.groupTasks(emptyList()),
    val isLoading: Boolean = true,
    val error: String? = null,
    val reminderErrors: Set<Long> = emptySet(),
)
