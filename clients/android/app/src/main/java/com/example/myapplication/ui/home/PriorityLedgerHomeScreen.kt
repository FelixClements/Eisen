package com.example.myapplication.ui.home

import android.content.Context
import android.view.accessibility.AccessibilityManager
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Archive
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material.icons.filled.PushPin
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Badge
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.ui.category.presentation
import com.example.myapplication.ui.theme.LedgerCategoryColor
import com.example.myapplication.ui.theme.MyApplicationTheme
import com.example.myapplication.ui.theme.ledgerCategoryColors
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.launch

private val LedgerCardOuterGutter = 16.dp
private val LedgerLeadingRailWidth = 48.dp
private val LedgerLeadingRailTextGap = 8.dp

@Composable
fun PriorityLedgerHomeRoute(
    viewModel: HomeViewModel,
    onOpenNewTask: (EisenhowerCategory) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsState()

    PriorityLedgerHomeScreen(
        uiState = uiState,
        onTaskCompletionChange = { task, completed ->
            viewModel.completeTask(task.id, completed)
        },
        onArchiveTask = { task -> viewModel.archiveTask(task.id) },
        onOpenNewTask = onOpenNewTask,
        modifier = modifier,
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PriorityLedgerHomeScreen(
    uiState: HomeUiState,
    onTaskCompletionChange: (Task, Boolean) -> Unit,
    onArchiveTask: (Task) -> Unit,
    onOpenNewTask: (EisenhowerCategory) -> Unit,
    modifier: Modifier = Modifier,
) {
    val sectionSpecs = remember { priorityLedgerSectionSpecs() }
    val now = remember { System.currentTimeMillis() }
    val visibleTasks = remember(uiState.groupedTasks, sectionSpecs) {
        sectionSpecs.flatMap { spec -> uiState.groupedTasks[spec.category].orEmpty() }
    }
    val taskIds = remember(visibleTasks) { visibleTasks.map { it.id } }
    val listState = rememberLazyListState()
    val coroutineScope = rememberCoroutineScope()
    val focusRequester = remember { FocusRequester() }
    val snackbarHostState = remember { SnackbarHostState() }
    val accessibilityManager = LocalContext.current
        .getSystemService(Context.ACCESSIBILITY_SERVICE) as? AccessibilityManager
    val isTouchExplorationEnabled = accessibilityManager?.isTouchExplorationEnabled == true
    var focusedTaskId by rememberSaveable { mutableStateOf<Long?>(null) }
    var showShortcutHelp by rememberSaveable { mutableStateOf(false) }
    var lastJumpedCategory by rememberSaveable { mutableStateOf<EisenhowerCategory?>(null) }

    val listIndexes = remember(uiState.groupedTasks, sectionSpecs) {
        buildListIndexMap(sectionSpecs, uiState.groupedTasks)
    }

    LaunchedEffect(taskIds) {
        if (focusedTaskId == null && lastJumpedCategory == null) {
            focusedTaskId = taskIds.firstOrNull()
        } else if (focusedTaskId != null && taskIds.none { taskId -> taskId == focusedTaskId }) {
            focusedTaskId = taskIds.firstOrNull()
        }
    }

    LaunchedEffect(isTouchExplorationEnabled) {
        if (!isTouchExplorationEnabled) {
            focusRequester.requestFocus()
        }
    }

    fun scrollToTask(task: Task) {
        listIndexes.taskIndexes[task.id]?.let { index ->
            coroutineScope.launch { listState.animateScrollToItem(index) }
        }
    }

    fun moveFocus(delta: Int): Boolean {
        if (visibleTasks.isEmpty()) return false
        val currentIndex = visibleTasks.indexOfFirst { it.id == focusedTaskId }.let { index ->
            if (index == -1) 0 else index
        }
        val newIndex = (currentIndex + delta).coerceIn(0, visibleTasks.lastIndex)
        val task = visibleTasks[newIndex]
        focusedTaskId = task.id
        scrollToTask(task)
        return true
    }

    fun jumpToCategory(category: EisenhowerCategory): Boolean {
        lastJumpedCategory = category
        listIndexes.sectionIndexes[category]?.let { index ->
            coroutineScope.launch { listState.animateScrollToItem(index) }
        }
        val firstTask = uiState.groupedTasks[category].orEmpty().firstOrNull()
        focusedTaskId = firstTask?.id
        return true
    }

    fun defaultNewTaskCategory(): EisenhowerCategory {
        val focusedTaskCategory = visibleTasks
            .firstOrNull { task -> task.id == focusedTaskId }
            ?.eisenhowerCategory

        return focusedTaskCategory ?: lastJumpedCategory ?: EisenhowerCategory.DO_NOW
    }

    fun openNewTask(): Boolean {
        onOpenNewTask(defaultNewTaskCategory())
        return true
    }

    fun archiveTask(task: Task): Boolean {
        onArchiveTask(task)
        coroutineScope.launch { snackbarHostState.showSnackbar("Archived") }
        return true
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("Today's Tasks") },
                actions = {
                    IconButton(onClick = { showShortcutHelp = true }) {
                        Icon(
                            imageVector = Icons.Filled.Keyboard,
                            contentDescription = "Keyboard shortcuts",
                        )
                    }
                },
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { openNewTask() },
                icon = { Icon(imageVector = Icons.Filled.Add, contentDescription = null) },
                text = { Text("[A] Add") },
                modifier = Modifier.semantics {
                    contentDescription = "Add task. Keyboard shortcut A"
                },
            )
        },
        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
    ) { innerPadding ->
        LazyColumn(
            state = listState,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .onPreviewKeyEvent { event ->
                    if (event.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
                    when (event.key) {
                        Key.J -> moveFocus(1)
                        Key.K -> moveFocus(-1)
                        Key.Q -> jumpToCategory(EisenhowerCategory.DO_NOW)
                        Key.W -> jumpToCategory(EisenhowerCategory.SCHEDULE)
                        Key.E -> jumpToCategory(EisenhowerCategory.DELEGATE_WAITING)
                        Key.R -> jumpToCategory(EisenhowerCategory.ELIMINATE_LATER)
                        Key.Spacebar -> {
                            visibleTasks.firstOrNull { it.id == focusedTaskId }?.let { task ->
                                onTaskCompletionChange(task, !task.isCompleted)
                                true
                            } ?: false
                        }
                        Key.Backspace -> {
                            visibleTasks.firstOrNull { it.id == focusedTaskId }?.let(::archiveTask) ?: false
                        }
                        Key.A -> openNewTask()
                        else -> false
                    }
                }
                .focusRequester(focusRequester)
                .focusable(),
            contentPadding = PaddingValues(start = 16.dp, top = 16.dp, end = 16.dp, bottom = 96.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item(key = "ledger-header") {
                LedgerSummary(uiState = uiState, now = now)
            }

            sectionSpecs.forEach { spec ->
                item(key = "section-${spec.category.name}") {
                    CategorySectionHeader(
                        spec = spec,
                        taskCount = uiState.groupedTasks[spec.category].orEmpty().size,
                    )
                }

                val tasks = uiState.groupedTasks[spec.category].orEmpty()
                if (tasks.isEmpty()) {
                    item(key = "empty-${spec.category.name}") {
                        EmptyCategoryCard()
                    }
                } else {
                    items(
                        items = tasks,
                        key = { task -> "${spec.category.name}-${task.id}" },
                    ) { task ->
                        PriorityTaskRow(
                            task = task,
                            now = now,
                            isFocused = task.id == focusedTaskId,
                            onCheckedChange = { checked -> onTaskCompletionChange(task, checked) },
                            onArchive = { archiveTask(task) },
                        )
                    }
                }
            }
        }
    }

    if (showShortcutHelp) {
        ShortcutHelpDialog(onDismiss = { showShortcutHelp = false })
    }
}

@Composable
private fun LedgerSummary(
    uiState: HomeUiState,
    now: Long,
) {
    val taskCount = uiState.activeTasks.size
    val urgentCount = uiState.activeTasks.count { task -> task.isUrgent }
    val overdueCount = uiState.activeTasks.count { task ->
        task.dueDate?.let { dueDate -> dueDate < now } == true
    }
    val summary = if (uiState.isLoading) {
        "Loading ledger..."
    } else {
        "${taskCount.countLabel("task", "tasks")} · " +
            "${urgentCount.countLabel("urgent", "urgent")} · " +
            overdueCount.countLabel("overdue", "overdue")
    }

    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.medium,
        color = MaterialTheme.colorScheme.surfaceContainerLow,
        contentColor = MaterialTheme.colorScheme.onSurface,
    ) {
        Text(
            text = summary,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun CategorySectionHeader(
    spec: LedgerSectionSpec,
    taskCount: Int,
) {
    val presentation = spec.category.presentation()
    val categoryColors = spec.category.ledgerCategoryColors()
    OutlinedCard(
        modifier = Modifier
            .fillMaxWidth()
            .semantics {
                heading()
                stateDescription =
                    "${taskCount.countLabel("task", "tasks")}. Keyboard shortcut ${presentation.shortcutLabel}"
            },
        colors = CardDefaults.outlinedCardColors(
            containerColor = categoryColors.container,
            contentColor = categoryColors.onContainer,
        ),
        border = BorderStroke(1.dp, categoryColors.outline),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = LedgerCardOuterGutter, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier.size(LedgerLeadingRailWidth),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = presentation.icon,
                    contentDescription = null,
                    modifier = Modifier.size(24.dp),
                    tint = categoryColors.onContainer,
                )
            }
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(start = LedgerLeadingRailTextGap),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    text = presentation.label,
                    style = MaterialTheme.typography.titleMedium,
                    color = categoryColors.onContainer,
                )
                Text(
                    text = spec.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = categoryColors.onContainer,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Keycap(label = presentation.shortcutLabel, categoryColors = categoryColors)
                Badge(
                    containerColor = categoryColors.accent,
                    contentColor = categoryColors.onAccent,
                ) {
                    Text(taskCount.toString())
                }
            }
        }
    }
}

@Composable
private fun PriorityTaskRow(
    task: Task,
    now: Long,
    isFocused: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    onArchive: () -> Unit,
) {
    val statusLine = taskStatusLine(task, now)

    OutlinedCard(
        modifier = Modifier
            .fillMaxWidth()
            .semantics {
                if (isFocused) {
                    stateDescription = "Selected for keyboard actions"
                }
            },
        border = if (isFocused) {
            BorderStroke(3.dp, MaterialTheme.colorScheme.primary)
        } else {
            CardDefaults.outlinedCardBorder()
        },
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier
                    .padding(start = LedgerCardOuterGutter)
                    .size(LedgerLeadingRailWidth),
                contentAlignment = Alignment.Center,
            ) {
                Checkbox(
                    checked = task.isCompleted,
                    onCheckedChange = onCheckedChange,
                    modifier = Modifier.semantics {
                        contentDescription = if (task.isCompleted) {
                            "Mark ${task.title} incomplete"
                        } else {
                            "Mark ${task.title} complete"
                        }
                    },
                )
            }
            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(
                        start = LedgerLeadingRailTextGap,
                        top = LedgerCardOuterGutter,
                        bottom = LedgerCardOuterGutter,
                    ),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    text = task.title,
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                statusLine?.let { line ->
                    Text(
                        text = line,
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            Row(
                modifier = Modifier.padding(end = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (task.isPinned) {
                    Icon(
                        imageVector = Icons.Filled.PushPin,
                        contentDescription = "Pinned",
                    )
                }
                IconButton(onClick = onArchive) {
                    Icon(
                        imageVector = Icons.Filled.Archive,
                        contentDescription = "Archive ${task.title}",
                    )
                }
            }
        }
    }
}

@Composable
private fun EmptyCategoryCard() {
    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            text = "No tasks yet.",
            modifier = Modifier.padding(
                horizontal = LedgerCardOuterGutter,
                vertical = LedgerCardOuterGutter,
            ),
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}

@Composable
private fun ShortcutHelpDialog(onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Keyboard shortcuts") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                ShortcutHelpRow(keys = "J / K", action = "Move through tasks")
                EisenhowerCategory.entries.forEach { category ->
                    val presentation = category.presentation()
                    ShortcutHelpRow(
                        keys = presentation.shortcutLabel,
                        action = "Jump to ${presentation.label}",
                    )
                }
                ShortcutHelpRow(keys = "Space", action = "Complete or uncomplete task")
                ShortcutHelpRow(keys = "Backspace", action = "Archive selected task")
                ShortcutHelpRow(keys = "A", action = "Open the New Task composer")
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text("Done")
            }
        },
    )
}

@Composable
private fun ShortcutHelpRow(
    keys: String,
    action: String,
) {
    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
    ) {
        ListItem(
            headlineContent = { Text(text = action, style = MaterialTheme.typography.bodyMedium) },
            leadingContent = { Keycap(label = keys) },
        )
    }
}

@Composable
private fun Keycap(
    label: String,
    categoryColors: LedgerCategoryColor? = null,
) {
    val containerColor = categoryColors?.accent ?: MaterialTheme.colorScheme.surfaceContainerHigh
    val contentColor = categoryColors?.onAccent ?: MaterialTheme.colorScheme.onSurface
    val outlineColor = categoryColors?.outline ?: MaterialTheme.colorScheme.outlineVariant

    Surface(
        modifier = Modifier.defaultMinSize(minWidth = 32.dp, minHeight = 32.dp),
        shape = MaterialTheme.shapes.extraSmall,
        color = containerColor,
        contentColor = contentColor,
        border = BorderStroke(1.dp, outlineColor),
    ) {
        Box(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
            )
        }
    }
}

private data class LedgerSectionSpec(
    val category: EisenhowerCategory,
    val description: String,
)

private data class LedgerListIndexes(
    val sectionIndexes: Map<EisenhowerCategory, Int>,
    val taskIndexes: Map<Long, Int>,
)

private fun buildListIndexMap(
    sectionSpecs: List<LedgerSectionSpec>,
    groupedTasks: Map<EisenhowerCategory, List<Task>>,
): LedgerListIndexes {
    var index = 1 // The first lazy item is the page summary.
    val sectionIndexes = mutableMapOf<EisenhowerCategory, Int>()
    val taskIndexes = mutableMapOf<Long, Int>()

    sectionSpecs.forEach { spec ->
        sectionIndexes[spec.category] = index
        index += 1

        val tasks = groupedTasks[spec.category].orEmpty()
        if (tasks.isEmpty()) {
            index += 1
        } else {
            tasks.forEach { task ->
                taskIndexes[task.id] = index
                index += 1
            }
        }
    }

    return LedgerListIndexes(sectionIndexes, taskIndexes)
}

private fun priorityLedgerSectionSpecs(): List<LedgerSectionSpec> = listOf(
    LedgerSectionSpec(
        category = EisenhowerCategory.DO_NOW,
        description = "Urgent and important",
    ),
    LedgerSectionSpec(
        category = EisenhowerCategory.SCHEDULE,
        description = "Important, not urgent",
    ),
    LedgerSectionSpec(
        category = EisenhowerCategory.DELEGATE_WAITING,
        description = "Urgent, not important",
    ),
    LedgerSectionSpec(
        category = EisenhowerCategory.ELIMINATE_LATER,
        description = "Not urgent, not important",
    ),
)

private fun taskStatusLine(task: Task, now: Long): String? {
    val parts = buildList {
        task.dueDate?.let { dueDate -> add(formatDueDate(dueDate, now)) }
        task.category?.takeIf { it.isNotBlank() }?.let { add(it) }
    }

    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

private fun formatDueDate(dueAtMillis: Long, now: Long): String {
    val todayStart = startOfDay(now)
    val day = 24L * 60L * 60L * 1000L
    val time = SimpleDateFormat("h:mm a", Locale.getDefault()).format(Date(dueAtMillis))

    return when {
        dueAtMillis < now -> "Overdue $time"
        dueAtMillis < todayStart + day -> "Due today $time"
        dueAtMillis < todayStart + (2 * day) -> "Due tomorrow $time"
        else -> "Due ${SimpleDateFormat("EEE, MMM d", Locale.getDefault()).format(Date(dueAtMillis))}"
    }
}

private fun startOfDay(millis: Long): Long = Calendar.getInstance().apply {
    timeInMillis = millis
    set(Calendar.HOUR_OF_DAY, 0)
    set(Calendar.MINUTE, 0)
    set(Calendar.SECOND, 0)
    set(Calendar.MILLISECOND, 0)
}.timeInMillis

private fun Int.countLabel(singular: String, plural: String): String =
    "$this ${if (this == 1) singular else plural}"

private val sampleNow = 1_800_000_000_000L

private fun sampleTasks(): List<Task> = listOf(
    Task(
        id = 1,
        title = "Pay electricity bill",
        description = null,
        isImportant = true,
        isUrgent = true,
        dueDate = sampleNow + (2L * 60L * 60L * 1000L),
        isCompleted = false,
        isArchived = false,
        isPinned = true,
        category = null,
        createdAt = sampleNow - 80_000L,
        updatedAt = sampleNow - 80_000L,
    ),
    Task(
        id = 2,
        title = "Finish assignment draft",
        description = null,
        isImportant = true,
        isUrgent = true,
        dueDate = sampleNow + (5L * 60L * 60L * 1000L),
        isCompleted = false,
        isArchived = false,
        isPinned = false,
        category = null,
        createdAt = sampleNow - 70_000L,
        updatedAt = sampleNow - 70_000L,
    ),
    Task(
        id = 3,
        title = "Study Kotlin Room relations",
        description = null,
        isImportant = true,
        isUrgent = false,
        dueDate = sampleNow + (2L * 24L * 60L * 60L * 1000L),
        isCompleted = false,
        isArchived = false,
        isPinned = false,
        category = "Deep work",
        createdAt = sampleNow - 60_000L,
        updatedAt = sampleNow - 60_000L,
    ),
    Task(
        id = 4,
        title = "Ask Mark about signed documents",
        description = null,
        isImportant = false,
        isUrgent = true,
        dueDate = sampleNow + (24L * 60L * 60L * 1000L),
        isCompleted = false,
        isArchived = false,
        isPinned = false,
        category = "Waiting",
        createdAt = sampleNow - 50_000L,
        updatedAt = sampleNow - 50_000L,
    ),
    Task(
        id = 5,
        title = "Watch saved keyboard review",
        description = null,
        isImportant = false,
        isUrgent = false,
        dueDate = null,
        isCompleted = false,
        isArchived = false,
        isPinned = false,
        category = null,
        createdAt = sampleNow - 40_000L,
        updatedAt = sampleNow - 40_000L,
    ),
)

@Preview(
    name = "Titan 2 Elite 1080x1200",
    widthDp = 540,
    heightDp = 600,
    showBackground = true,
)
@Composable
private fun PriorityLedgerHomeScreenTitanPreview() {
    val tasks = sampleTasks()
    MyApplicationTheme(darkTheme = true, dynamicColor = false) {
        PriorityLedgerHomeScreen(
            uiState = HomeUiState(
                activeTasks = tasks,
                groupedTasks = HomeTaskSorter.groupTasks(tasks),
                isLoading = false,
            ),
            onTaskCompletionChange = { _, _ -> },
            onArchiveTask = {},
            onOpenNewTask = {},
        )
    }
}

@Preview(name = "Priority Ledger light", showBackground = true)
@Composable
private fun PriorityLedgerHomeScreenLightPreview() {
    val tasks = sampleTasks()
    MyApplicationTheme(darkTheme = false, dynamicColor = false) {
        PriorityLedgerHomeScreen(
            uiState = HomeUiState(
                activeTasks = tasks,
                groupedTasks = HomeTaskSorter.groupTasks(tasks),
                isLoading = false,
            ),
            onTaskCompletionChange = { _, _ -> },
            onArchiveTask = {},
            onOpenNewTask = {},
        )
    }
}
