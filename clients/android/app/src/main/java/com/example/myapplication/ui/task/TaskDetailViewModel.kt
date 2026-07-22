package com.example.myapplication.ui.task

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface TaskDetailUiEvent {
    data class OperationFailed(val message: String) : TaskDetailUiEvent
}

class TaskDetailViewModel(
    private val taskId: Long,
    private val repository: TaskRepository,
) : ViewModel() {
    private val _events = MutableSharedFlow<TaskDetailUiEvent>()
    val events: SharedFlow<TaskDetailUiEvent> = _events.asSharedFlow()

    val task: StateFlow<Task?> = repository.getTaskById(taskId)
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = null
        )

    fun updateTask(updatedTask: Task) {
        viewModelScope.launch {
            runCatching {
                repository.updateTask(updatedTask)
            }.onFailure {
                _events.emit(TaskDetailUiEvent.OperationFailed("Failed to save changes"))
            }
        }
    }

    class Factory(
        private val taskId: Long,
        private val repository: TaskRepository,
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return TaskDetailViewModel(taskId, repository) as T
        }
    }
}
