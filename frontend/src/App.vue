<template>
  <v-app>
    <v-app-bar app dark color="primary">
      <v-toolbar-title class="headline text-uppercase">
        <router-link to="/" class="white--text">Factorio Servers Statistics</router-link>
      </v-toolbar-title>
      <v-spacer></v-spacer>
    </v-app-bar>

    <v-content>
      <v-container fluid fill-height>
        <router-view />
      </v-container>
    </v-content>
  </v-app>
</template>

<style>
  a {
    text-decoration: none !important;
  }

  .rich-text img {
    vertical-align: middle;
    margin: 0 0.4vw;
  }
</style>

<script>
  export default {
    name: 'App',
    data: () => ({
      //
    }),
  };
</script>
