package com.example.myapplication

import android.os.Bundle
import android.content.Intent
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.example.myapplication.data.LocalTaskRepository
import com.example.myapplication.data.local.DatabaseProvider
import com.example.myapplication.data.reminder.TaskReminderNotifications
import com.example.myapplication.data.reminder.WorkManagerTaskReminderScheduler
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.data.reminder.TaskReminderWorker
import com.example.myapplication.ui.history.HistoryViewModel
import com.example.myapplication.ui.home.HomeViewModel
import com.example.myapplication.ui.navigation.PriorityLedgerApp
import com.example.myapplication.ui.theme.MyApplicationTheme

class MainActivity : ComponentActivity() {
    private var initialTaskId by mutableLongStateOf(-1L)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        TaskReminderNotifications.createChannel(applicationContext)
        val database = DatabaseProvider.getDatabase(applicationContext)
        val repository = LocalTaskRepository(database.taskDao())
        val reminderScheduler = WorkManagerTaskReminderScheduler(applicationContext)
        val homeViewModel = ViewModelProvider(
            this,
            HomeViewModelFactory(repository, reminderScheduler),
        )[HomeViewModel::class.java]

        val historyViewModel = ViewModelProvider(
            this,
            HistoryViewModelFactory(repository, reminderScheduler),
        )[HistoryViewModel::class.java]

        initialTaskId = intent.getLongExtra(TaskReminderWorker.EXTRA_TASK_ID, -1L)

        setContent {
            MyApplicationTheme {
                PriorityLedgerApp(
                    repository = repository,
                    homeViewModel = homeViewModel,
                    historyViewModel = historyViewModel,
                    initialTaskId = initialTaskId,
                    onTaskIdHandled = { initialTaskId = -1L },
                )
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        val taskId = intent.getLongExtra(TaskReminderWorker.EXTRA_TASK_ID, -1L)
        if (taskId != -1L) {
            initialTaskId = taskId
        }
    }
}

private class HistoryViewModelFactory(
    private val repository: TaskRepository,
    private val reminderScheduler: TaskReminderScheduler,
) : ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(HistoryViewModel::class.java)) {
            return HistoryViewModel(repository, reminderScheduler) as T
        }

        throw IllegalArgumentException("Unknown ViewModel class: ${modelClass.name}")
    }
}

private class HomeViewModelFactory(
    private val repository: TaskRepository,
    private val reminderScheduler: TaskReminderScheduler,
) : ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(HomeViewModel::class.java)) {
            return HomeViewModel(repository, reminderScheduler) as T
        }

        throw IllegalArgumentException("Unknown ViewModel class: ${modelClass.name}")
    }
}
