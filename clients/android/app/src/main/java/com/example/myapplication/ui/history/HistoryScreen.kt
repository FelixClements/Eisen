package com.example.myapplication.ui.history

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Restore
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.example.myapplication.R
import com.example.myapplication.domain.Task
import com.example.myapplication.ui.theme.MyApplicationTheme
import com.example.myapplication.ui.util.DateTimeUtils
import kotlin.time.Duration.Companion.minutes
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.launch

private val HistoryContentGutter = 16.dp

@Composable
fun HistoryRoute(
    viewModel: HistoryViewModel,
    onOpenNavigationDrawer: () -> Unit,
    onOpenTaskDetail: (Task) -> Unit,
    onOpenKeyboardShortcuts: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsState()

    HistoryScreen(
        uiState = uiState,
        events = viewModel.events,
        onUncompleteTask = viewModel::uncompleteTask,
        onUnarchiveTask = viewModel::unarchiveTask,
        onOpenNavigationDrawer = onOpenNavigationDrawer,
        onOpenTaskDetail = onOpenTaskDetail,
        onOpenKeyboardShortcuts = onOpenKeyboardShortcuts,
        modifier = modifier,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryScreen(
    uiState: HistoryUiState,
    events: Flow<HistoryUiEvent> = emptyFlow(),
    onUncompleteTask: (Long) -> Unit,
    onUnarchiveTask: (Long) -> Unit,
    onOpenNavigationDrawer: () -> Unit = {},
    onOpenTaskDetail: (Task) -> Unit = {},
    onOpenKeyboardShortcuts: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    var selectedTab by rememberSaveable { mutableIntStateOf(0) }
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }

    LaunchedEffect(Unit) {
        while (true) {
            delay(1.minutes)
            now = System.currentTimeMillis()
        }
    }

    val tabs = listOf(
        stringResource(R.string.status_completed),
        stringResource(R.string.status_archived)
    )
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()

    val taskRestoredLedgerMsg = stringResource(R.string.task_restored_ledger)
    val taskRestoredArchiveMsg = stringResource(R.string.task_restored_archive)

    LaunchedEffect(events) {
        events.collect { event ->
            when (event) {
                is HistoryUiEvent.OperationFailed -> snackbarHostState.showSnackbar(event.message)
            }
        }
    }

    Scaffold(
        modifier = modifier
            .fillMaxSize()
            .onPreviewKeyEvent { event ->
                if (event.type == KeyEventType.KeyDown) {
                    when (event.key) {
                        Key.M -> {
                            onOpenNavigationDrawer()
                            true
                        }
                        Key.Slash -> {
                            if (event.isShiftPressed) {
                                onOpenKeyboardShortcuts()
                                true
                            } else false
                        }
                        else -> false
                    }
                } else false
            },
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.history_title)) },
                navigationIcon = {
                    IconButton(onClick = onOpenNavigationDrawer) {
                        Icon(
                            imageVector = Icons.Filled.Menu,
                            contentDescription = stringResource(R.string.open_nav_drawer),
                        )
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            TabRow(selectedTabIndex = selectedTab) {
                tabs.forEachIndexed { index, title ->
                    Tab(
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        text = { Text(title) },
                    )
                }
            }

            when (selectedTab) {
                0 -> CompletedTab(
                    tasks = uiState.completedTasks,
                    now = now,
                    onUncomplete = { taskId ->
                        onUncompleteTask(taskId)
                        coroutineScope.launch {
                            snackbarHostState.showSnackbar(taskRestoredLedgerMsg)
                        }
                    },
                    onTaskClick = onOpenTaskDetail,
                )
                1 -> ArchiveTab(
                    tasks = uiState.archivedTasks,
                    now = now,
                    onUnarchive = { taskId ->
                        onUnarchiveTask(taskId)
                        coroutineScope.launch {
                            snackbarHostState.showSnackbar(taskRestoredArchiveMsg)
                        }
                    },
                    onTaskClick = onOpenTaskDetail,
                )
            }
        }
    }
}

@Composable
private fun CompletedTab(
    tasks: List<Task>,
    now: Long,
    onUncomplete: (Long) -> Unit,
    onTaskClick: (Task) -> Unit,
) {
    if (tasks.isEmpty()) {
        HistoryEmptyState(message = stringResource(R.string.history_completed_empty))
    } else {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(HistoryContentGutter),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(items = tasks, key = { it.id }) { task ->
                HistoryTaskRow(
                    task = task,
                    now = now,
                    actionLabel = stringResource(R.string.undo), // Or "Uncomplete"
                    actionContentDescription = stringResource(R.string.incomplete_task, task.title),
                    onAction = { onUncomplete(task.id) },
                    onClick = { onTaskClick(task) },
                )
            }
        }
    }
}

@Composable
private fun ArchiveTab(
    tasks: List<Task>,
    now: Long,
    onUnarchive: (Long) -> Unit,
    onTaskClick: (Task) -> Unit,
) {
    if (tasks.isEmpty()) {
        HistoryEmptyState(message = stringResource(R.string.history_archive_empty))
    } else {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(HistoryContentGutter),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(items = tasks, key = { it.id }) { task ->
                HistoryTaskRow(
                    task = task,
                    now = now,
                    actionLabel = stringResource(R.string.done), // Or "Restore"
                    actionContentDescription = stringResource(R.string.restore_task, task.title),
                    onAction = { onUnarchive(task.id) },
                    onClick = { onTaskClick(task) },
                )
            }
        }
    }
}

@Composable
private fun HistoryTaskRow(
    task: Task,
    now: Long,
    actionLabel: String,
    actionContentDescription: String,
    onAction: () -> Unit,
    onClick: () -> Unit = {},
) {
    val statusLine = buildList {
        task.dueDate?.let { dueDate ->
            add(DateTimeUtils.formatDueDate(dueDate, now))
        }
        task.category?.takeIf { it.isNotBlank() }?.let { add(it) }
    }.joinToString(" · ")

    OutlinedCard(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(
                        start = HistoryContentGutter,
                        top = HistoryContentGutter,
                        bottom = HistoryContentGutter,
                    ),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    text = task.title,
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (statusLine.isNotBlank()) {
                    Text(
                        text = statusLine,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
            IconButton(
                onClick = onAction,
                modifier = Modifier.semantics {
                    contentDescription = actionContentDescription
                },
            ) {
                Icon(
                    imageVector = Icons.Filled.Restore,
                    contentDescription = null,
                )
            }
        }
    }
}

@Composable
private fun HistoryEmptyState(message: String) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Preview(name = "History completed populated", widthDp = 400)
@Composable
private fun HistoryCompletedPreview() {
    MyApplicationTheme(darkTheme = false, dynamicColor = false) {
        HistoryScreen(
            uiState = HistoryUiState(
                completedTasks = listOf(
                    Task(
                        id = 1, title = "Pay electricity bill", description = null,
                        isImportant = true, isUrgent = true, dueDate = null,
                        isCompleted = true, isArchived = false, isPinned = false,
                        category = null, createdAt = 100L, updatedAt = 200L,
                    ),
                    Task(
                        id = 2, title = "Finish assignment draft", description = null,
                        isImportant = true, isUrgent = true, dueDate = null,
                        isCompleted = true, isArchived = false, isPinned = false,
                        category = "School", createdAt = 100L, updatedAt = 200L,
                    ),
                ),
                archivedTasks = emptyList(),
            ),
            onUncompleteTask = {},
            onUnarchiveTask = {},
        )
    }
}

@Preview(name = "History archive populated", widthDp = 400)
@Composable
private fun HistoryArchivePreview() {
    MyApplicationTheme(darkTheme = false, dynamicColor = false) {
        HistoryScreen(
            uiState = HistoryUiState(
                completedTasks = emptyList(),
                archivedTasks = listOf(
                    Task(
                        id = 3, title = "Old report", description = null,
                        isImportant = false, isUrgent = false, dueDate = null,
                        isCompleted = true, isArchived = true, isPinned = false,
                        category = null, createdAt = 50L, updatedAt = 150L,
                    ),
                ),
            ),
            onUncompleteTask = {},
            onUnarchiveTask = {},
        )
    }
}

@Preview(name = "History empty", widthDp = 400)
@Composable
private fun HistoryEmptyPreview() {
    MyApplicationTheme(darkTheme = false, dynamicColor = false) {
        HistoryScreen(
            uiState = HistoryUiState(),
            onUncompleteTask = {},
            onUnarchiveTask = {},
        )
    }
}
