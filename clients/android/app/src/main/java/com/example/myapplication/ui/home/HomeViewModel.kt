package com.example.myapplication.ui.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.NoOpTaskReminderScheduler
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface HomeUiEvent {
    data class TaskCompleted(val task: Task, val isCompleted: Boolean) : HomeUiEvent
    data class TaskArchived(val task: Task) : HomeUiEvent
    data class OperationFailed(val message: String) : HomeUiEvent
}

class HomeViewModel(
    private val repository: TaskRepository,
    private val reminderScheduler: TaskReminderScheduler = NoOpTaskReminderScheduler,
    private val clock: () -> Long = { System.currentTimeMillis() },
) : ViewModel() {
    private val retryTrigger = MutableStateFlow(0L)
    private val reminderErrors = MutableStateFlow<Set<Long>>(emptySet())
    private val searchQuery = MutableStateFlow("")
    private val _events = MutableSharedFlow<HomeUiEvent>()
    val events = _events.asSharedFlow()

    private val lastScheduledReminders = mutableMapOf<Long, Long>()

    val uiState: StateFlow<HomeUiState> = combine(
        retryTrigger.flatMapLatest {
            searchQuery.flatMapLatest { query ->
                if (query.isBlank()) repository.observeActiveTasks()
                else repository.searchTasks(query)
            }
        },
        reminderErrors
    ) { tasks, errors ->
        HomeUiState(
            activeTasks = tasks,
            groupedTasks = HomeTaskSorter.groupTasks(tasks),
            isLoading = false,
            error = null,
            reminderErrors = errors,
        )
    }
        .catch { e ->
            emit(HomeUiState(isLoading = false, error = e.message ?: "Failed to load tasks"))
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.Eagerly,
            initialValue = HomeUiState(),
        )

    fun retry() {
        retryTrigger.value = clock()
    }

    fun search(query: String) {
        searchQuery.value = query
    }

    init {
        reconcileReminders()
    }

    private fun reconcileReminders() {
        viewModelScope.launch {
            combine(
                repository.observeAllTasks(),
                reminderScheduler.getScheduledTaskIds()
            ) { tasks, scheduledIds ->
                val errors = mutableSetOf<Long>()
                tasks.forEach { task ->
                    val isScheduled = scheduledIds.contains(task.id)
                    val reminderAt = task.reminderAt ?: 0L
                    val shouldBeScheduled = !task.isCompleted && !task.isArchived &&
                        reminderAt > clock()

                    val needsScheduling = shouldBeScheduled &&
                        (!isScheduled || lastScheduledReminders[task.id] != reminderAt)

                    if (needsScheduling) {
                        reminderScheduler.schedule(task.id, reminderAt)
                            .onSuccess { lastScheduledReminders[task.id] = reminderAt }
                            .onFailure { errors.add(task.id) }
                    } else if (!shouldBeScheduled && isScheduled) {
                        reminderScheduler.cancel(task.id)
                            .onSuccess { lastScheduledReminders.remove(task.id) }
                            .onFailure { errors.add(task.id) }
                    }
                }
                reminderErrors.value = errors
            }.catch {
                // Non-fatal, reconciliation will be retried on next change
            }.collect { }
        }
    }

    fun addTask(
        title: String,
        category: EisenhowerCategory,
        description: String? = null,
        dueDate: Long? = null,
        reminderAt: Long? = null,
        taskCategory: String? = null,
        onResult: (Result<Long>) -> Unit = {},
    ) {
        val trimmedTitle = title.trim()
        if (trimmedTitle.isBlank()) return

        viewModelScope.launch {
            runCatching {
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
                    category = taskCategory,
                    createdAt = now,
                    updatedAt = now,
                )
                val taskId = repository.addTask(task)
                task.reminderAt?.let { reminderAt ->
                    reminderScheduler.schedule(taskId, reminderAt)
                }
                taskId
            }.onSuccess { taskId ->
                onResult(Result.success(taskId))
            }.onFailure { error ->
                onResult(Result.failure(error))
            }
        }
    }

    fun completeTask(task: Task, isCompleted: Boolean = true) {
        viewModelScope.launch {
            runCatching {
                repository.completeTask(task.id, isCompleted)
            }.onSuccess {
                _events.emit(HomeUiEvent.TaskCompleted(task, isCompleted))
            }.onFailure {
                _events.emit(HomeUiEvent.OperationFailed("Failed to complete task"))
            }
        }
    }

    fun archiveTask(task: Task) {
        viewModelScope.launch {
            runCatching {
                repository.archiveTask(task.id)
            }.onSuccess {
                _events.emit(HomeUiEvent.TaskArchived(task))
            }.onFailure {
                _events.emit(HomeUiEvent.OperationFailed("Failed to archive task"))
            }
        }
    }

    fun unarchiveTask(id: Long) {
        viewModelScope.launch {
            repository.unarchiveTask(id)
        }
    }

    fun moveTask(id: Long, category: EisenhowerCategory) {
        viewModelScope.launch {
            repository.moveTask(id, category)
        }
    }
}
