package com.example.myapplication.ui.navigation

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.ViewList
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.ui.history.HistoryRoute
import com.example.myapplication.ui.history.HistoryViewModel
import com.example.myapplication.ui.home.HomeViewModel
import com.example.myapplication.ui.home.PriorityLedgerHomeRoute
import com.example.myapplication.ui.settings.SettingsScreen
import com.example.myapplication.ui.task.NewTaskScreen
import com.example.myapplication.ui.task.TaskDetailScreen
import com.example.myapplication.ui.task.TaskDetailViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.launch

import androidx.compose.ui.res.stringResource
import com.example.myapplication.R

object PriorityLedgerRoutes {
    const val LEDGER = "ledger"
    const val HISTORY = "history"
    const val SETTINGS = "settings"
    const val KEYBOARD_SHORTCUTS = "keyboard-shortcuts"
    const val NEW_TASK = "new-task/{defaultCategory}"
    const val TASK_DETAIL = "task-detail/{taskId}"

    fun newTask(defaultCategory: EisenhowerCategory): String =
        "new-task/${defaultCategory.name}"

    fun taskDetail(taskId: Long): String = "task-detail/$taskId"
}

private const val DrawerItemTagPrefix = "navigation-drawer-item-"

@Composable
fun PriorityLedgerApp(
    repository: TaskRepository,
    homeViewModel: HomeViewModel,
    historyViewModel: HistoryViewModel,
    initialTaskId: Long = -1L,
    onTaskIdHandled: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val navController = rememberNavController()

    LaunchedEffect(initialTaskId) {
        if (initialTaskId != -1L) {
            navController.navigate(PriorityLedgerRoutes.taskDetail(initialTaskId))
            onTaskIdHandled()
        }
    }
    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val coroutineScope = rememberCoroutineScope()
    val backStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = backStackEntry?.destination?.route ?: PriorityLedgerRoutes.LEDGER
    // Match against the route pattern, including parameters for NEW_TASK
    val isNewTaskRoute = currentRoute.startsWith("new-task/")
    val drawerAvailable = !isNewTaskRoute

    LaunchedEffect(drawerAvailable) {
        if (!drawerAvailable && drawerState.isOpen) {
            drawerState.close()
        }
    }

    BackHandler(enabled = drawerAvailable && drawerState.isOpen) {
        coroutineScope.launch { drawerState.close() }
    }

    fun navigateToTopLevel(route: String) {
        coroutineScope.launch {
            navController.navigate(route) {
                popUpTo(PriorityLedgerRoutes.LEDGER) {
                    saveState = true
                }
                launchSingleTop = true
                restoreState = true
            }
            drawerState.close()
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        gesturesEnabled = drawerAvailable,
        drawerContent = {
            LedgerNavigationDrawer(
                currentRoute = currentRoute,
                drawerStateDescription = if (drawerState.isOpen) "Open" else "Closed",
                onDestinationSelected = ::navigateToTopLevel,
            )
        },
    ) {
        NavHost(
            navController = navController,
            startDestination = PriorityLedgerRoutes.LEDGER,
            modifier = modifier,
        ) {
            composable(PriorityLedgerRoutes.LEDGER) {
                PriorityLedgerHomeRoute(
                    viewModel = homeViewModel,
                    onOpenNavigationDrawer = {
                        coroutineScope.launch { drawerState.open() }
                    },
                    onOpenNewTask = { category ->
                        navController.navigate(PriorityLedgerRoutes.newTask(category))
                    },
                    onOpenTaskDetail = { task ->
                        navController.navigate(PriorityLedgerRoutes.taskDetail(task.id))
                    },
                    onOpenKeyboardShortcuts = {
                        navController.navigate(PriorityLedgerRoutes.KEYBOARD_SHORTCUTS)
                    },
                )
            }
            composable(PriorityLedgerRoutes.HISTORY) {
                HistoryRoute(
                    viewModel = historyViewModel,
                    onOpenNavigationDrawer = {
                        coroutineScope.launch { drawerState.open() }
                    },
                    onOpenTaskDetail = { task ->
                        navController.navigate(PriorityLedgerRoutes.taskDetail(task.id))
                    },
                    onOpenKeyboardShortcuts = {
                        navController.navigate(PriorityLedgerRoutes.KEYBOARD_SHORTCUTS)
                    },
                )
            }
            composable(PriorityLedgerRoutes.SETTINGS) {
                SettingsScreen(
                    onOpenNavigationDrawer = {
                        coroutineScope.launch { drawerState.open() }
                    },
                    onOpenKeyboardShortcuts = {
                        navController.navigate(PriorityLedgerRoutes.KEYBOARD_SHORTCUTS)
                    },
                )
            }
            composable(PriorityLedgerRoutes.KEYBOARD_SHORTCUTS) {
                KeyboardShortcutsScreen(
                    onOpenNavigationDrawer = {
                        coroutineScope.launch { drawerState.open() }
                    },
                )
            }
            composable(
                route = PriorityLedgerRoutes.TASK_DETAIL,
                arguments = listOf(navArgument("taskId") { type = NavType.LongType }),
            ) { entry ->
                val taskId = entry.arguments?.getLong("taskId") ?: -1L
                val detailViewModel: TaskDetailViewModel = viewModel(
                    factory = TaskDetailViewModel.Factory(taskId, repository)
                )
                TaskDetailScreen(
                    viewModel = detailViewModel,
                    onBack = { navController.popBackStack() },
                    onOpenKeyboardShortcuts = {
                        navController.navigate(PriorityLedgerRoutes.KEYBOARD_SHORTCUTS)
                    }
                )
            }
            composable(
                route = PriorityLedgerRoutes.NEW_TASK,
                arguments = listOf(navArgument("defaultCategory") { type = NavType.StringType }),
            ) { entry ->
                val defaultCategory = entry.arguments
                    ?.getString("defaultCategory")
                    ?.let { name -> EisenhowerCategory.entries.find { it.name == name } }
                    ?: EisenhowerCategory.DO_NOW

                NewTaskScreen(
                    defaultCategory = defaultCategory,
                    onSaveTask = { title, category, notes, dueDate, reminderAt, taskCategory, onDone ->
                        homeViewModel.addTask(
                            title,
                            category,
                            description = notes,
                            dueDate = dueDate,
                            reminderAt = reminderAt,
                            taskCategory = taskCategory,
                            onResult = onDone,
                        )
                    },
                    onTaskSaved = { navController.popBackStack() },
                    onCancel = { navController.popBackStack() },
                )
            }
        }
    }
}

@Composable
private fun LedgerNavigationDrawer(
    currentRoute: String,
    drawerStateDescription: String,
    onDestinationSelected: (String) -> Unit,
) {
    val destinations = listOf(
        NavigationDestination(stringResource(R.string.home_title), PriorityLedgerRoutes.LEDGER, Icons.Filled.ViewList),
        NavigationDestination(stringResource(R.string.history_title), PriorityLedgerRoutes.HISTORY, Icons.Filled.History),
        NavigationDestination(stringResource(R.string.settings_title), PriorityLedgerRoutes.SETTINGS, Icons.Filled.Settings),
        NavigationDestination(
            stringResource(R.string.keyboard_shortcuts_title),
            PriorityLedgerRoutes.KEYBOARD_SHORTCUTS,
            Icons.Filled.Keyboard,
        ),
    )

    val navDrawerDescription = stringResource(R.string.nav_drawer_description)

    ModalDrawerSheet(
        modifier = Modifier
            .semantics {
                contentDescription = navDrawerDescription
                stateDescription = drawerStateDescription
            },
    ) {
        Text(
            text = stringResource(R.string.app_name),
            modifier = Modifier.padding(horizontal = 28.dp, vertical = 24.dp),
            style = MaterialTheme.typography.titleLarge,
        )
        destinations.forEach { destination ->
            NavigationDrawerItem(
                label = { Text(destination.label) },
                selected = currentRoute == destination.route,
                onClick = { onDestinationSelected(destination.route) },
                icon = {
                    Icon(
                        imageVector = destination.icon,
                        contentDescription = "${destination.label} destination",
                    )
                },
                modifier = Modifier
                    .padding(horizontal = 12.dp)
                    .testTag("$DrawerItemTagPrefix${destination.route}"),
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun KeyboardShortcutsScreen(onOpenNavigationDrawer: () -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.keyboard_shortcuts_title)) },
                navigationIcon = {
                    DrawerNavigationButton(onClick = onOpenNavigationDrawer)
                },
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Text(
                text = stringResource(R.string.global_shortcuts),
                modifier = Modifier.padding(top = 16.dp, start = 16.dp),
                style = MaterialTheme.typography.titleMedium,
            )
            ShortcutListItem("?", stringResource(R.string.show_help_shortcut))
            ShortcutListItem("M", stringResource(R.string.open_drawer_shortcut))
            Text(
                text = stringResource(R.string.ledger_shortcuts),
                modifier = Modifier.padding(top = 16.dp, start = 16.dp),
                style = MaterialTheme.typography.titleMedium,
            )
            ShortcutListItem("J / K", stringResource(R.string.move_focus_shortcut))
            ShortcutListItem("Q / W / E / R", stringResource(R.string.jump_category_shortcut))
            ShortcutListItem("Space", stringResource(R.string.complete_uncomplete_shortcut))
            ShortcutListItem("Backspace", stringResource(R.string.archive_selected_shortcut))
            ShortcutListItem("A", stringResource(R.string.open_new_task_shortcut))
            Text(
                text = stringResource(R.string.new_task_shortcuts),
                modifier = Modifier.padding(top = 16.dp, start = 16.dp),
                style = MaterialTheme.typography.titleMedium,
            )
            ShortcutListItem("Alt + Q / W / E / R", stringResource(R.string.select_category_shortcut))
        }
    }
}

@Composable
private fun ShortcutListItem(keys: String, action: String) {
    ListItem(
        headlineContent = { Text(action) },
        supportingContent = { Text(keys) },
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun DrawerNavigationButton(onClick: () -> Unit) {
    IconButton(onClick = onClick) {
        Icon(
            imageVector = Icons.Filled.Menu,
            contentDescription = stringResource(R.string.open_nav_drawer),
        )
    }
}

private data class NavigationDestination(
    val label: String,
    val route: String,
    val icon: ImageVector,
)
