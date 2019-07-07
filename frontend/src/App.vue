<template>
  <v-app>
    <v-app-bar app dark color="primary">
      <v-toolbar-title class="headline text-uppercase">
        <router-link to="/" class="white--text">Factorio Servers Statistics</router-link>
      </v-toolbar-title>
      <v-spacer></v-spacer>
      <v-btn
        text
        href="https://github.com/vuetifyjs/vuetify/releases/latest"
        target="_blank"
      >
        <span class="mr-2">Latest Release</span>
        <v-icon>mdi-open-in-new</v-icon>
      </v-btn>
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
</style>

<script>
  export default {
    name: 'App',
    data: () => ({
      //
    }),
  };
</script>
