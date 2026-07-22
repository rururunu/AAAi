import { listChatSessions, openSettings } from "@/services/ipc";

export interface SlashCommand {
    command: string;
    label: string;
    description: string;
}

export type SlashCommandAction =
    | "close"
    | "openHistory"
    | "openModel"
    | "openWorkspace"
    | "enterPlan"
    | "clearInput"
    | null;

export const slashCommands: SlashCommand[] = [
    {
        command: "/history",
        label: "history",
        description: "打开历史对话",
    },
    {
        command: "/model",
        label: "model",
        description: "切换对话模型",
    },
    {
        command: "/plan",
        label: "plan",
        description: "进入计划模式（只读探索）",
    },
    {
        command: "/settings",
        label: "settings",
        description: "打开设置",
    },
    {
        command: "/work",
        label: "work",
        description: "快速切换工作区",
    },
    {
        command: "/exit",
        label: "exit",
        description: "关闭当前会话",
    },
    {
        command: "/clear",
        label: "clear",
        description: "清空当前会话输入与本地草稿",
    },
];

export async function executeSlashCommand(
    command: string,
): Promise<SlashCommandAction> {
    switch (command) {
        case "/history":
            return "openHistory";
        case "/model":
            return "openModel";
        case "/plan":
            return "enterPlan";
        case "/settings":
            try {
                await openSettings();
            } catch (error) {
                console.error("Failed to open settings:", error);
            }
            return null;
        case "/work":
            return "openWorkspace";
        case "/exit":
            return "close";
        case "/clear":
            return "clearInput";
        default:
            return null;
    }
}

export async function fetchChatSessions() {
    try {
        const response = await listChatSessions();
        return response.sessions ?? [];
    } catch (error) {
        console.error("list_chat_sessions failed:", error);
        return [];
    }
}
