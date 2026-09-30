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
  ...common.en,
  ...auth.en,
  ...dashboard.en,
  ...networks.en,
  ...networkDetail.en,
  ...networkForm.en,
  ...nodes.en,
  ...nodeJoin.en,
  ...credentials.en,
  ...logs.en,
  ...settings.en,
  ...components.en,
};
