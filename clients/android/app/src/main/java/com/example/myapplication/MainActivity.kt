package com.example.myapplication

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.example.myapplication.data.LocalTaskRepository
import com.example.myapplication.data.local.DatabaseProvider
import com.example.myapplication.data.reminder.TaskReminderNotifications
import com.example.myapplication.data.reminder.WorkManagerTaskReminderScheduler
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.ui.home.HomeViewModel
import com.example.myapplication.ui.navigation.PriorityLedgerApp
import com.example.myapplication.ui.theme.MyApplicationTheme

class MainActivity : ComponentActivity() {
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

        setContent {
            MyApplicationTheme {
                PriorityLedgerApp(homeViewModel = homeViewModel)
            }
        }
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
