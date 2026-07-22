package com.example.myapplication.ui.history

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface HistoryUiEvent {
    data class OperationFailed(val message: String) : HistoryUiEvent
}

class HistoryViewModel(
    private val repository: TaskRepository,
    private val reminderScheduler: TaskReminderScheduler,
) : ViewModel() {
    private val _events = MutableSharedFlow<HistoryUiEvent>()
    val events: SharedFlow<HistoryUiEvent> = _events.asSharedFlow()

    val uiState: StateFlow<HistoryUiState> = combine(
        repository.observeCompletedTasks(),
        repository.observeArchivedTasks(),
    ) { completed, archived ->
        HistoryUiState(
            completedTasks = completed,
            archivedTasks = archived,
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.Eagerly,
        initialValue = HistoryUiState(),
    )

    fun uncompleteTask(id: Long) {
        viewModelScope.launch {
            runCatching {
                repository.completeTask(id, isCompleted = false)
                repository.getTaskById(id).first()?.let { task ->
                    task.reminderAt?.let { reminderAt ->
                        if (reminderAt > System.currentTimeMillis()) {
                            reminderScheduler.schedule(id, reminderAt).getOrThrow()
                        }
                    }
                }
            }.onFailure {
                _events.emit(HistoryUiEvent.OperationFailed("Failed to restore task"))
            }
        }
    }

    fun unarchiveTask(id: Long) {
        viewModelScope.launch {
            runCatching {
                repository.unarchiveTask(id)
                val task = repository.getTaskById(id).first() ?: return@runCatching
                // Schedule future reminder only if the unarchived task is active (not completed).
                if (!task.isCompleted) {
                    task.reminderAt?.let { reminderAt ->
                        if (reminderAt > System.currentTimeMillis()) {
                            reminderScheduler.schedule(id, reminderAt).getOrThrow()
                        }
                    }
                }
            }.onFailure {
                _events.emit(HistoryUiEvent.OperationFailed("Failed to restore task"))
            }
        }
    }
}

data class HistoryUiState(
    val completedTasks: List<Task> = emptyList(),
    val archivedTasks: List<Task> = emptyList(),
)
