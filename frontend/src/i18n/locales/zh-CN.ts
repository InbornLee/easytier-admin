import common from "../namespaces/common";
import auth from "../namespaces/auth";
import dashboard from "../namespaces/dashboard";
import networks from "../namespaces/networks";
import networkDetail from "../namespaces/networkDetail";
import networkForm from "../namespaces/networkForm";
import nodes from "../namespaces/nodes";
import nodeJoin from "../namespaces/nodeJoin";
import credentials from "../namespaces/credentials";
import logs from "../namespaces/logs";
import settings from "../namespaces/settings";
import components from "../namespaces/components";

export default {
  ...common.zhCN,
  ...auth.zhCN,
  ...dashboard.zhCN,
  ...networks.zhCN,
  ...networkDetail.zhCN,
  ...networkForm.zhCN,
  ...nodes.zhCN,
  ...nodeJoin.zhCN,
  ...credentials.zhCN,
  ...logs.zhCN,
  ...settings.zhCN,
  ...components.zhCN,
};
