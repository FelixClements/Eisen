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
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Archive
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.PushPin
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Badge
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.example.myapplication.R
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.ui.category.presentation
import com.example.myapplication.ui.components.TopBarHeight
import com.example.myapplication.ui.theme.LedgerCategoryColor
import com.example.myapplication.ui.theme.MyApplicationTheme
import com.example.myapplication.ui.theme.ledgerCategoryColors
import com.example.myapplication.ui.util.DateTimeUtils
import kotlin.time.Duration.Companion.minutes
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.launch

private val LedgerCardOuterGutter = 16.dp
private val LedgerLeadingRailWidth = 48.dp
private val LedgerLeadingRailTextGap = 8.dp

private const val LedgerTaskListTag = "ledger-task-list"

@Composable
fun PriorityLedgerHomeRoute(
    viewModel: HomeViewModel,
    onOpenNewTask: (EisenhowerCategory) -> Unit,
    onOpenTaskDetail: (Task) -> Unit,
    onOpenKeyboardShortcuts: () -> Unit = {},
    onOpenNavigationDrawer: () -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsState()

    PriorityLedgerHomeScreen(
        uiState = uiState,
        events = viewModel.events,
        onTaskCompletionChange = viewModel::completeTask,
        onArchiveTask = viewModel::archiveTask,
        onUnarchiveTask = { task -> viewModel.unarchiveTask(task.id) },
        onRetry = viewModel::retry,
        onOpenNewTask = onOpenNewTask,
        onOpenTaskDetail = onOpenTaskDetail,
        onOpenKeyboardShortcuts = onOpenKeyboardShortcuts,
        onOpenNavigationDrawer = onOpenNavigationDrawer,
        onSearch = viewModel::search,
        modifier = modifier,
    )
}

@Composable
fun PriorityLedgerHomeScreen(
    uiState: HomeUiState,
    events: SharedFlow<HomeUiEvent>,
    onTaskCompletionChange: (Task, Boolean) -> Unit,
    onArchiveTask: (Task) -> Unit,
    onUnarchiveTask: (Task) -> Unit = {},
    onRetry: () -> Unit = {},
    onOpenNewTask: (EisenhowerCategory) -> Unit,
    onOpenTaskDetail: (Task) -> Unit = {},
    onOpenKeyboardShortcuts: () -> Unit = {},
    onOpenNavigationDrawer: () -> Unit = {},
    onSearch: (String) -> Unit = {},
    modifier: Modifier = Modifier,
) {
    val sectionSpecs = remember { priorityLedgerSectionSpecs() }
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    var isSearchActive by rememberSaveable { mutableStateOf(false) }
    var searchQuery by rememberSaveable { mutableStateOf("") }
    val searchFocusRequester = remember { FocusRequester() }

    LaunchedEffect(isSearchActive) {
        if (isSearchActive) {
            searchFocusRequester.requestFocus()
        }
    }

    LaunchedEffect(Unit) {
        while (true) {
            delay(1.minutes)
            now = System.currentTimeMillis()
        }
    }

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
    var lastJumpedCategory by rememberSaveable { mutableStateOf<EisenhowerCategory?>(null) }

    val listIndexes = remember(uiState.groupedTasks, sectionSpecs) {
        buildListIndexMap(sectionSpecs, uiState.groupedTasks)
    }

    val taskCompletedMsg = stringResource(R.string.task_completed)
    val taskArchivedMsg = stringResource(R.string.task_archived)
    val undoLabel = stringResource(R.string.undo)

    LaunchedEffect(events) {
        events.collect { event ->
            when (event) {
                is HomeUiEvent.TaskCompleted -> {
                    if (event.isCompleted) {
                        val result = snackbarHostState.showSnackbar(
                            message = taskCompletedMsg,
                            actionLabel = undoLabel,
                            duration = SnackbarDuration.Short,
                        )
                        if (result == SnackbarResult.ActionPerformed) {
                            onTaskCompletionChange(event.task, false)
                        }
                    }
                }
                is HomeUiEvent.TaskArchived -> {
                    val result = snackbarHostState.showSnackbar(
                        message = taskArchivedMsg,
                        actionLabel = undoLabel,
                        duration = SnackbarDuration.Short,
                    )
                    if (result == SnackbarResult.ActionPerformed) {
                        onUnarchiveTask(event.task)
                    }
                }
                is HomeUiEvent.OperationFailed -> {
                    snackbarHostState.showSnackbar(event.message)
                }
            }
        }
    }

    LaunchedEffect(taskIds) {
        if (focusedTaskId == null && lastJumpedCategory == null) {
            focusedTaskId = taskIds.firstOrNull()
        } else if (focusedTaskId != null && taskIds.none { it == focusedTaskId }) {
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
        focusedTaskId = firstTask?.id ?: focusedTaskId
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

    fun completeTask(task: Task, completed: Boolean) {
        onTaskCompletionChange(task, completed)
    }

    fun archiveTask(task: Task): Boolean {
        onArchiveTask(task)
        return true
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        containerColor = MaterialTheme.colorScheme.background,
        contentWindowInsets = WindowInsets.navigationBars,
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { openNewTask() },
                icon = { Icon(imageVector = Icons.Filled.Add, contentDescription = null) },
                text = { Text(stringResource(R.string.add_task_fab_shortcut)) },
                modifier = Modifier.semantics {
                    contentDescription = "Add task. Keyboard shortcut A"
                },
            )
        },
        snackbarHost = { SnackbarHost(hostState = snackbarHostState) },
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            val hasTasks = uiState.activeTasks.isNotEmpty()

            when {
                uiState.isLoading -> {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = TopBarHeight),
                        contentAlignment = Alignment.Center,
                    ) {
                        CircularProgressIndicator()
                    }
                }

                uiState.error != null -> {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = TopBarHeight),
                        contentAlignment = Alignment.Center,
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text(
                                text = uiState.error ?: "",
                                style = MaterialTheme.typography.bodyLarge,
                                color = MaterialTheme.colorScheme.error,
                            )
                            Button(
                                onClick = onRetry,
                                modifier = Modifier.padding(top = 16.dp),
                            ) {
                                Text(stringResource(R.string.retry))
                            }
                        }
                    }
                }

                !hasTasks -> {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(top = TopBarHeight),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = if (isSearchActive) stringResource(R.string.search_empty, searchQuery) else stringResource(R.string.ledger_empty),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }

                else -> {
                    LazyColumn(
                        state = listState,
                        modifier = Modifier
                            .fillMaxSize()
                            .onPreviewKeyEvent { event ->
                                if (event.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
                                when (event.key) {
                                    Key.J -> moveFocus(1)
                                    Key.K -> moveFocus(-1)
                                    Key.Q -> jumpToCategory(EisenhowerCategory.DO_NOW)
                                    Key.W -> jumpToCategory(EisenhowerCategory.SCHEDULE)
                                    Key.E -> jumpToCategory(EisenhowerCategory.DELEGATE_WAITING)
                                    Key.R -> jumpToCategory(EisenhowerCategory.ELIMINATE_LATER)
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
                                    Key.Spacebar -> {
                                        visibleTasks.firstOrNull { it.id == focusedTaskId }?.let { task ->
                                            completeTask(task, !task.isCompleted)
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
                            .focusable()
                            .testTag(LedgerTaskListTag),
                        contentPadding = PaddingValues(start = 16.dp, top = TopBarHeight, end = 16.dp, bottom = 96.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
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
                                    EmptyCategoryCard(spec.category)
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
                                        hasReminderError = uiState.reminderErrors.contains(task.id),
                                        onCheckedChange = { checked -> onTaskCompletionChange(task, checked) },
                                        onArchive = { onArchiveTask(task) },
                                        onClick = { onOpenTaskDetail(task) },
                                    )
                                }
                            }
                        }
                    }
                }
            }

            LedgerTopOverlay(
                isSearchActive = isSearchActive,
                query = searchQuery,
                focusRequester = searchFocusRequester,
                onQueryChange = {
                    searchQuery = it
                    onSearch(it)
                },
                onCloseSearch = {
                    isSearchActive = false
                    searchQuery = ""
                    onSearch("")
                },
                onOpenSearch = { isSearchActive = true },
                onOpenNavigationDrawer = onOpenNavigationDrawer,
                modifier = Modifier.align(Alignment.TopCenter),
            )
        }
    }
}

@Composable
private fun LedgerTopOverlay(
    isSearchActive: Boolean,
    query: String,
    focusRequester: FocusRequester,
    onQueryChange: (String) -> Unit,
    onCloseSearch: () -> Unit,
    onOpenSearch: () -> Unit,
    onOpenNavigationDrawer: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (isSearchActive) {
        Row(
            modifier = modifier
                .fillMaxWidth()
                .height(TopBarHeight)
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onCloseSearch) {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = stringResource(R.string.back_button),
                )
            }
            OutlinedTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier
                    .weight(1f)
                    .focusRequester(focusRequester),
                placeholder = { Text(stringResource(R.string.search_placeholder)) },
                singleLine = true,
                colors = TextFieldDefaults.colors(
                    focusedContainerColor = Color.Transparent,
                    unfocusedContainerColor = Color.Transparent,
                    disabledContainerColor = Color.Transparent,
                ),
            )
        }
    } else {
        Row(
            modifier = modifier
                .fillMaxWidth()
                .height(TopBarHeight)
                .padding(horizontal = 4.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onOpenNavigationDrawer) {
                Icon(
                    imageVector = Icons.Filled.Menu,
                    contentDescription = stringResource(R.string.open_nav_drawer),
                )
            }
            IconButton(onClick = onOpenSearch) {
                Icon(
                    imageVector = Icons.Filled.Search,
                    contentDescription = stringResource(R.string.search_tasks),
                )
            }
        }
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
                    "$taskCount tasks. Keyboard shortcut ${presentation.shortcutLabel}"
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
    hasReminderError: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    onArchive: () -> Unit,
    onClick: () -> Unit,
) {
    val statusLine = taskStatusLine(task, now)
    val completeLabel = stringResource(R.string.complete_task, task.title)
    val incompleteLabel = stringResource(R.string.incomplete_task, task.title)
    val archiveLabel = stringResource(R.string.archive_task, task.title)

    OutlinedCard(
        onClick = onClick,
        modifier = Modifier
            .fillMaxWidth()
            .semantics {
                if (isFocused) {
                    stateDescription = "Selected for keyboard actions"
                }
                customActions = listOf(
                    CustomAccessibilityAction(if (task.isCompleted) incompleteLabel else completeLabel) {
                        onCheckedChange(!task.isCompleted)
                        true
                    },
                    CustomAccessibilityAction(archiveLabel) {
                        onArchive()
                        true
                    }
                )
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
                        contentDescription = if (task.isCompleted) incompleteLabel else completeLabel
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
                if (hasReminderError) {
                    Icon(
                        imageVector = Icons.Filled.Warning,
                        contentDescription = stringResource(R.string.reminder_error),
                        tint = MaterialTheme.colorScheme.error,
                    )
                }
                if (task.isPinned) {
                    Icon(
                        imageVector = Icons.Filled.PushPin,
                        contentDescription = stringResource(R.string.pinned_task),
                    )
                }
                IconButton(onClick = onArchive) {
                    Icon(
                        imageVector = Icons.Filled.Archive,
                        contentDescription = archiveLabel,
                    )
                }
            }
        }
    }
}

@Composable
private fun EmptyCategoryCard(category: EisenhowerCategory) {
    val guidance = when (category) {
        EisenhowerCategory.DO_NOW -> stringResource(R.string.do_now_guidance)
        EisenhowerCategory.SCHEDULE -> stringResource(R.string.schedule_guidance)
        EisenhowerCategory.DELEGATE_WAITING -> stringResource(R.string.delegate_guidance)
        EisenhowerCategory.ELIMINATE_LATER -> stringResource(R.string.eliminate_guidance)
    }
    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            text = guidance,
            modifier = Modifier.padding(
                horizontal = LedgerCardOuterGutter,
                vertical = LedgerCardOuterGutter,
            ),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
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
    var index = 0
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
        task.dueDate?.let { dueDate -> add(DateTimeUtils.formatDueDate(dueDate, now)) }
        task.category?.takeIf { it.isNotBlank() }?.let { add(it) }
    }

    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

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
            events = MutableSharedFlow(),
            onTaskCompletionChange = { _, _ -> },
            onArchiveTask = {},
            onOpenTaskDetail = {},
            onOpenNewTask = {},
        )
    }
}

@Preview(name = "Home light", showBackground = true)
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
            events = MutableSharedFlow(),
            onTaskCompletionChange = { _, _ -> },
            onArchiveTask = {},
            onOpenTaskDetail = {},
            onOpenNewTask = {},
        )
    }
}
