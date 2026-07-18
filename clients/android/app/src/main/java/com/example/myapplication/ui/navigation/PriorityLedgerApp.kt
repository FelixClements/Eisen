package com.example.myapplication.ui.navigation

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.ui.home.HomeViewModel
import com.example.myapplication.ui.home.PriorityLedgerHomeRoute
import com.example.myapplication.ui.task.NewTaskScreen

private const val LEDGER_ROUTE = "ledger"
private const val NEW_TASK_ROUTE = "new-task/{defaultCategory}"

@Composable
fun PriorityLedgerApp(
    homeViewModel: HomeViewModel,
    modifier: Modifier = Modifier,
) {
    val navController = rememberNavController()

    NavHost(
        navController = navController,
        startDestination = LEDGER_ROUTE,
        modifier = modifier,
    ) {
        composable(LEDGER_ROUTE) {
            PriorityLedgerHomeRoute(
                viewModel = homeViewModel,
                onOpenNewTask = { category ->
                    navController.navigate("new-task/${category.name}")
                },
            )
        }
        composable(
            route = NEW_TASK_ROUTE,
            arguments = listOf(navArgument("defaultCategory") { type = NavType.StringType }),
        ) { backStackEntry ->
            val defaultCategory = backStackEntry.arguments
                ?.getString("defaultCategory")
                ?.let { name -> EisenhowerCategory.entries.find { it.name == name } }
                ?: EisenhowerCategory.DO_NOW

            NewTaskScreen(
                defaultCategory = defaultCategory,
                onSaveTask = { title, category, notes, dueDate, reminderAt ->
                    homeViewModel.addTask(
                        title,
                        category,
                        description = notes,
                        dueDate = dueDate,
                        reminderAt = reminderAt,
                    )
                    navController.popBackStack()
                },
                onCancel = { navController.popBackStack() },
            )
        }
    }
}
