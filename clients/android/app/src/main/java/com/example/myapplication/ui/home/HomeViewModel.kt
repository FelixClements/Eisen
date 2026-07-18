package com.example.myapplication.ui.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.NoOpTaskReminderScheduler
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

class HomeViewModel(
    private val repository: TaskRepository,
    private val reminderScheduler: TaskReminderScheduler = NoOpTaskReminderScheduler,
    private val clock: () -> Long = { System.currentTimeMillis() },
) : ViewModel() {
    val uiState: StateFlow<HomeUiState> = repository.observeActiveTasks()
        .map { tasks ->
            HomeUiState(
                activeTasks = tasks,
                groupedTasks = HomeTaskSorter.groupTasks(tasks),
                isLoading = false,
            )
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.Eagerly,
            initialValue = HomeUiState(),
        )

    init {
        viewModelScope.launch {
            repository.observeActiveTasks().first().forEach(::scheduleFutureReminder)
        }
    }

    fun addTask(
        title: String,
        category: EisenhowerCategory,
        description: String? = null,
        dueDate: Long? = null,
        reminderAt: Long? = null,
    ) {
        val trimmedTitle = title.trim()
        if (trimmedTitle.isBlank()) return

        viewModelScope.launch {
            val now = clock()
            val task = Task(
                title = trimmedTitle,
                description = description,
                isImportant = category.isImportant,
                isUrgent = category.isUrgent,
                dueDate = dueDate,
                reminderAt = reminderAt,
                isCompleted = false,
                isArchived = false,
                isPinned = false,
                category = null,
                createdAt = now,
                updatedAt = now,
            )
            val taskId = repository.addTask(task)
            task.reminderAt?.let { reminderAt ->
                reminderScheduler.schedule(taskId, reminderAt)
            }
        }
    }

    fun completeTask(id: Long, isCompleted: Boolean = true) {
        viewModelScope.launch {
            repository.completeTask(id, isCompleted)
            if (isCompleted) {
                reminderScheduler.cancel(id)
            } else {
                repository.getTaskById(id).first()?.let(::scheduleFutureReminder)
            }
        }
    }

    fun archiveTask(id: Long) {
        viewModelScope.launch {
            repository.archiveTask(id)
            reminderScheduler.cancel(id)
        }
    }

    fun moveTask(id: Long, category: EisenhowerCategory) {
        viewModelScope.launch {
            repository.moveTask(id, category)
        }
    }

    private fun scheduleFutureReminder(task: Task) {
        val reminderAt = task.reminderAt ?: return
        if (!task.isCompleted && !task.isArchived && reminderAt > clock()) {
            reminderScheduler.schedule(task.id, reminderAt)
        }
    }
}
