// The same cockpit opened on a view or a panel, so each can be judged on the canvas.

import Fusion from "./Fusion";

export const FusionMethods = () => <Fusion initialView="methods" />;
export const FusionSettings = () => <Fusion initialView="settings" />;
export const FusionNotifications = () => <Fusion initialPanel="notifications" />;
export const FusionProfile = () => <Fusion initialPanel="profile" />;
export const FusionChat = () => <Fusion initialTab="Chat" />;
